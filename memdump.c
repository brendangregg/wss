// Pulls the memory of a process into a structure like:
// pid/virtual-start/page-offset.mem
//
// Note: this needs to be run as root because:
// https://www.kernel.org/doc/Documentation/vm/pagemap.txt

#include <stdio.h>
#include <sys/types.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdint.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <math.h>
#include <errno.h>
#include <sys/ptrace.h>
#include <sys/wait.h>

#define BASE_PATH "/tmp/raw-mem"
#define PAGE_SIZE 4096
#define PATHSIZE   128

void dump_pages(int memfd, char *base_path, uint64_t addr, uint64_t page_offset, uint64_t pages) {
    char out_path[PATHSIZE];
    void *mem_data = malloc(pages * PAGE_SIZE);
    int out_fd;

    sprintf(out_path, "%s/0x%lx:%lu:%lu.mem", base_path, addr, page_offset, pages);
    out_fd = open(out_path, O_RDWR | O_CREAT | O_TRUNC, (mode_t)0666);

    if(lseek(memfd, addr + (page_offset * PAGE_SIZE), SEEK_SET) < 0) {
        printf("Failed to seek to %lu\n", addr + (page_offset * PAGE_SIZE));
        exit(1);
    }
    printf("Writing %lu bytes to %s\n", 4096UL * pages, out_path);
    if(read(memfd, mem_data, pages * PAGE_SIZE) != pages * PAGE_SIZE) {
        printf("Didn't read enought data...\n");
        exit(1);
    }

    write(out_fd, mem_data, pages * PAGE_SIZE);
    close(out_fd);
    free(mem_data);
}

// Find contiguous offsets.
// For each offset block create file "a-b.mem"
void dump_memory(pid_t pid, uint64_t base_addr, uint64_t *offsets, uint64_t offsets_size) {
    uint64_t offset;
    char mem_path[PATHSIZE];
    int mem_fd;
    char addr_path[PATHSIZE];
    uint64_t block_start;
    uint64_t block_size = 0;
    
    
    sprintf(addr_path, "%s/%i", BASE_PATH, pid);
    mkdir(addr_path, 0777);

    sprintf(mem_path, "/proc/%d/mem", pid);

    mem_fd = open(mem_path, O_RDONLY);

    ptrace(PTRACE_ATTACH, pid, NULL, NULL);
    waitpid(pid, NULL, 0);

    for(offset = 0; offset < offsets_size; ++offset) {
        if(block_size == 0) { // start a new block
            block_start = offsets[offset];
            block_size = 1;
        }
        if (offset == (offsets_size - 1) // am I the last one?
                // or am I the start of a new block?
                || offsets[offset + 1] > (block_start + block_size)) {
            dump_pages(mem_fd, addr_path, base_addr, block_start, block_size);
            block_size = 0;
        } else {
            block_size += 1;
        }
    }

    ptrace(PTRACE_DETACH, pid, NULL, NULL);
    //char output_path[100];
    //int output_fd;

}

void get_largest_memory_block(pid_t pid, uint64_t *address, uint64_t *size) {
    FILE *mapsfile;
    char mapspath[PATHSIZE];
    char line[500];
    uint64_t mapstart, mapend;

    // read virtual mappings
    if (sprintf(mapspath, "/proc/%d/maps", pid) < 0) {
        printf("Can't allocate memory. Exiting.");
        exit(1);
    }
    if ((mapsfile = fopen(mapspath, "r")) == NULL) {
        perror("Can't read maps file");
        exit(2);
    }

    while (fgets(line, sizeof (line), mapsfile) != NULL) {
        sscanf(line, "%lx-%lx", &mapstart, &mapend);
        if (mapend - mapstart > *size) {
            *size = mapend - mapstart;
            *address = mapstart;
        }
    }

    fclose(mapsfile);

}

// returns size of the valid pages array
uint64_t valid_pages(pid_t pid, uint64_t address, uint64_t size, uint64_t **pages) {
    char pagemap_path[100];
    int pagemap_fd;
    uint64_t offset;
    uint64_t *pagemap;
    uint64_t valid_pages_count = 0;
    int64_t bytes_read;
    uint64_t num_pages = (uint64_t) ceil((float)size / 4096);

    pagemap = malloc(num_pages * 8); // 8 bytes per pagemap entry.
    *pages = malloc(num_pages * sizeof(**pages)); // worst case when all pages are valid.
    
    sprintf(pagemap_path, "/proc/%i/pagemap", pid);
    pagemap_fd = open(pagemap_path, O_RDONLY);
    if(pagemap_fd < 0) {
        printf("Error opening %s\n", pagemap_path);
    } else {
        printf("Opened %s\n", pagemap_path);
    }
    printf("Seeking to %lu for fd %i\n", (address / 4096) * 8, pagemap_fd);
    if(lseek(pagemap_fd, (address / 4096) * 8, SEEK_SET) < 0) {
        printf("Can't seek to address in pagemap_fd\n");
    }
    printf("Trying to read %lu bytes\n", num_pages * 8);
    bytes_read = read(pagemap_fd, pagemap, num_pages * 8);
    printf("Read returned: %li\n", bytes_read);

    for(offset = 0; offset < num_pages; offset += 1) {
        if(pagemap[offset] & 0xFFFFFFF) {
            (*pages)[valid_pages_count++] = offset;
        }
    }

    return valid_pages_count;

}

int main(int argc, char **argv) {
    uint64_t target_address;
    uint64_t target_size = 0;
    uint64_t *valid_pages_arr;
    uint64_t valid_pages_count;
    //int fd;
    pid_t pid;

    if (argc < 1)
    {
        fprintf(stderr, "Missing arguments\n");
        return 2;
    }
    pid = atoi(argv[1]);
    get_largest_memory_block(pid, &target_address, &target_size);
    printf("Found biggest memory section 0x%lx with size %lu\n", target_address, target_size);
    valid_pages_count = valid_pages(pid, target_address, target_size, &valid_pages_arr);
    printf("Process has %lu valid pages.\n", valid_pages_count);
    dump_memory(pid, target_address, valid_pages_arr, valid_pages_count);

    // TODO: pause tracee using ptrace

    //get_largest_address_block(pid, &target_address, &target_size);
    //dump_memory(pid, target_address, target_size);
    //close(fd);
}
