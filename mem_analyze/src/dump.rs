use std::fs::File;
use std::io;
use std::io::{BufReader, BufRead, Write, Read, Seek, SeekFrom};
use sysinfo::SystemExt;
use byteorder::{ByteOrder, LittleEndian};
use std::{thread, time};
use chrono::Utc;
//use nix::sys::{ptrace, wait, signal};
//use nix::unistd::Pid;

// only interested in segments with at least 1000 pages
const SEGMENT_THRESHOLD: usize = 1000 * 4096;

// hmm, this is hard-coding 64-bit systems in here...
// TODO: detect user space end from current system address size.
// https://lwn.net/Articles/738975/
const USERSPACE_END: usize = 0xffff800000000000;

const PAGE_SIZE: usize = 4096;

// One bit per page; bit number if PFN.
const IDLE_BITMAP_PATH: &str = "/sys/kernel/mm/page_idle/bitmap";

const KPAGEFLAGS_PATH: &str = "/proc/kpageflags";
const KPAGEFLAGS_BIT_BUDDY: u8 = 10;

// Without a process ID means get the memory activity for the whole host.
// Note it doesn't analyze page contents, only type and activity.
pub fn get_host_memory(sleep: u64, inspect_ram: bool) -> Result<super::ProcessMemory, std::io::Error> {
    set_idlemap()?;
    //ptrace::cont(nix_pid, None);
    debug!("Sleeping {} seconds", sleep);
    thread::sleep(time::Duration::from_secs(sleep));
    let snapshot_time = Utc::now();
    //signal::kill(nix_pid, signal::Signal::SIGSTOP);
    let idlemap = load_idlemap()?;
    Ok(super::ProcessMemory {
        timestamp: snapshot_time,
        segments: get_physical_segments()?.iter().map(|segment|
            super::Segment {
                addr_start: 0,
                page_flags: get_kpageflags(segment).unwrap().into_iter().enumerate().map(|(pfn_idx, pfn_flags)| {
                    let active_page_add = get_active_add(pfn_idx as u64, &idlemap);
                    let zero_page_add: u64 = match inspect_ram {
                        true => match get_pfn_content(pfn_idx) {
                            Ok(content) => match content.iter().all(|&x| x == 0) {
                                true => 1 << super::ZERO_PAGE_BIT,
                                false => 0,
                            },
                            Err(e) => panic!("Got error: {:?}", e),
                        },
                        false => 0
                    };
                    (pfn_flags & !(1 << super::ACTIVE_PAGE_BIT))
                        + active_page_add
                        + zero_page_add
                }).collect(),
            }
        ).collect(),
    })
}

pub fn get_memory(pid: i32, sleep: u64) -> Result<super::ProcessMemory, std::io::Error> {
    //let nix_pid = Pid::from_raw(pid);
    //ptrace::attach(nix_pid);
    //wait::waitpid(nix_pid, None);
    //ptrace::detach(nix_pid);
    set_idlemap()?;
    //ptrace::cont(nix_pid, None);
    debug!("Sleeping {} seconds", sleep);
    thread::sleep(time::Duration::from_secs(sleep));
    let snapshot_time = Utc::now();
    //signal::kill(nix_pid, signal::Signal::SIGSTOP);
    let idlemap = load_idlemap()?;
    let segments: Vec<Segment> = get_virtual_segments(pid)?.into_iter()
        .filter(|s| s.start_address < USERSPACE_END && s.size >= SEGMENT_THRESHOLD)
        .collect();
    debug!("Process has {} (filtered segments", segments.len());
    let mut process_memory = super::ProcessMemory {
        timestamp: snapshot_time,
        segments : Vec::with_capacity(segments.len()),
    };
    let start_time = Utc::now();
    for segment in segments {
        let pagemap: Vec<u64> = get_pagemap(pid, &segment)?;
        debug!("Pagemap for segment at {} with size {} has len {}", segment.start_address, segment.size, pagemap.len());
        //let all_page_data = get_page_content(pid, segment.start_address)?;
        let mut data_slice: Option<Vec<u8>> = None;
        let mut data_slice_offset = 0;
        let page_flags: Vec<u64> = pagemap.iter().enumerate().map(|(page_idx, pagemap_word)|
            if pagemap_word & 1 << 63 == 0 {
                data_slice = None; //end of contiguous; clear for next mapped page.
                return pagemap_word.clone();
            } else {
                if pagemap_word & 1 << 62 != 0 {
                    data_slice = None; //end of contiguous; clear for next mapped page.
                    return pagemap_word.clone();

                } else {
                    if data_slice == None {
                        let page_range = contiguous_mapped_length(&pagemap[page_idx..]);
                        assert!(page_range > 0); // debugging; remove once happy with algorithm.
                        data_slice_offset = 0;
                        data_slice = Some(get_page_content(pid, segment.start_address + (page_idx * PAGE_SIZE), page_range).unwrap());
                    }
                    let page_data = match data_slice {
                        None => panic!("We were supposed to pre-read this but didn't...."),
                        Some(ref data_slice) => data_slice[(data_slice_offset* PAGE_SIZE)..((data_slice_offset+1) * PAGE_SIZE)].to_vec(),
                    };
                    data_slice_offset += 1;

                    let zero_page_add: u64 = match page_data.iter().all(|&x| x == 0) {
                        true => 1 << super::ZERO_PAGE_BIT,
                        false => 0,
                    };

                    // Bits 0-54  page frame number (PFN) if present
                    let active_page_add = get_active_add(pagemap_word & 0x7FFFFFFFFFFFFF, &idlemap);
                    // Zero the PFN; were going to use it to store other data resembling kpageflags
                    return (pagemap_word & !0x7FFFFFFFFFFFFF)
                            + zero_page_add + active_page_add;
                }
            }
        ).collect();
        process_memory.segments.push(super::Segment {
            addr_start: segment.start_address,
            page_flags: page_flags,
        });
    }
    debug!("Finished dumping segments in {} ms", (Utc::now() - start_time).num_milliseconds());
    Ok(process_memory)
}

fn get_active_add(pfn: u64, idlemap: &[u8]) -> u64 {
    return match idlemap[pfn as usize / 8] & 1 << pfn % 8 == 0 {
        true => 1 << super::ACTIVE_PAGE_BIT,
        false => 0,
    };
}

// Given an array slice of pagemap entries, where the starting element is a resident entry,
// returns how long the contiguous segment of resident entries is.
fn contiguous_mapped_length(pagemap: &[u64]) -> usize {
    for (idx, entry) in pagemap.iter().enumerate() {
        if entry & 1 << 63 == 0 {
            assert!(idx > 0); // we only expect this to be called when at a valid slice.
            return idx;
        }
    }
    return pagemap.len() // they're all valid!
}

struct Segment {
    pub start_address: usize,
    pub size: usize,
}

fn get_pagemap(pid: i32, segment: &Segment) -> std::io::Result<Vec<u64>> {
    return read_segment_data_from_file(segment, &format!("/proc/{}/pagemap", pid));
}

fn get_kpageflags(segment: &Segment) -> std::io::Result<Vec<u64>> {
    return read_segment_data_from_file(segment, KPAGEFLAGS_PATH);
}

fn read_segment_data_from_file(segment: &Segment, file_path: &str) -> std::io::Result<Vec<u64>> {
    let start_time = Utc::now();
    assert_eq!(segment.start_address % PAGE_SIZE, 0);
    // This is why we need to run the program as root
    // https://www.kernel.org/doc/Documentation/vm/pagemap.txt
    let mut file = File::open(file_path)?;
    // 64-bits = 8 bytes per page
    file.seek(SeekFrom::Start((segment.start_address / PAGE_SIZE) as u64 * 8))?;
    let mut data_bytes: Vec<u8> = Vec::with_capacity((segment.size / PAGE_SIZE) * 8);
    data_bytes.resize((segment.size / PAGE_SIZE ) * 8, 0);
    file.read_exact(data_bytes.as_mut_slice())?;
    assert_eq!(data_bytes.len() % 8, 0);
    let mut data_words: Vec<u64> = Vec::with_capacity(data_bytes.len() / 8);
    data_words.resize(data_bytes.len() / 8, 0);
    LittleEndian::read_u64_into(&data_bytes, &mut data_words);
    debug!("Loaded {} in {} ms", file_path, (Utc::now() - start_time).num_milliseconds());
    Ok(data_words)
}

fn get_virtual_segments(pid: i32) -> Result<Vec<Segment>, io::Error> {
    let mut segments: Vec<Segment> = Vec::new();
    let file = File::open(format!("/proc/{}/maps", pid))?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if let Ok((a, b)) = scan_fmt!(&line, "{x}-{x}", [hex usize], [hex usize]) {
            segments.push(Segment {
                start_address: a,
                size: b - a,
            })
        } else {
            error!("Unable to parse maps line: {}", line);
        }
    }
    Ok(segments)
}

fn get_physical_segments() -> Result<Vec<Segment>, io::Error> {
    let mut segments: Vec<Segment> = Vec::new();
    let file = File::open("/proc/iomem")?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.contains("System RAM") {
            if let Ok((a, b)) = scan_fmt!(&line, "{x}-{x}", [hex usize], [hex usize]) {
                segments.push(Segment {
                    start_address: a,
                    size: b - a,
                })
            } else {
                error!("Unable to parse maps line: {}", line);
            }
        }
    }
    Ok(segments)
}

fn set_idlemap() -> std::io::Result<()> {
    let start_time = Utc::now();
    let idle_bitmap_data: Vec<u8> = vec![0xff; 4096];
    let mut file = File::create(IDLE_BITMAP_PATH)?;
    let mut write_counter: usize = 0;
    loop {
        match file.write(&idle_bitmap_data) {
            Ok(wrote) => write_counter += wrote,
            Err(_e) => break,
        }
    }
    if write_counter * 8 < system_ram_pages() {
        error!("Fatal: unable to set sufficient idlemap pages. Only set {} bytes", write_counter);
        std::process::exit(1);
    }
    debug!("Idlemap set in {} ms", (Utc::now() - start_time).num_milliseconds());
    Ok(())
}

fn load_idlemap() -> std::io::Result<Vec<u8>> {
    let start_time = Utc::now();
    // kinda weird, but what we're doing here is 8-bits per page, but actually dividing by 7
    // to have the capacity a bit bigger, seeing as we seem to fill about 1.1 times the number
    // of idlemap bits as we have physical pages.
    let mut idlemap: Vec<u8> = Vec::with_capacity(system_ram_pages() / 7);
    let mut file = File::open(IDLE_BITMAP_PATH)?;
    file.read_to_end(&mut idlemap)?;
    debug!("Idlemap loaded in {} ms", (Utc::now() - start_time).num_milliseconds());
    Ok(idlemap)
}

fn get_page_content(pid: i32, page_addr_start: usize, pages: usize) -> std::io::Result<Vec<u8>> {
    let mut mem_file = File::open(format!("/proc/{}/mem", pid))?;
    mem_file.seek(SeekFrom::Start(page_addr_start as u64))?;
    let mut mem: Vec<u8> = Vec::with_capacity(pages * PAGE_SIZE);
    mem.resize(pages * PAGE_SIZE, 0); // why do I have to do this...?
    mem_file.read_exact(mem.as_mut_slice())?;
    Ok(mem)
}

fn get_pfn_content(pfn: usize) -> std::io::Result<Vec<u8>> {
    let mut mem_file = File::open("/dev/mem")?;
    mem_file.seek(SeekFrom::Start((pfn * PAGE_SIZE) as u64))?;
    let mut mem: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
    mem.resize(PAGE_SIZE, 0); // why do I have to do this...?
    mem_file.read_exact(mem.as_mut_slice())?;
    Ok(mem)
}

fn system_ram_pages() -> usize {
    system_ram_bytes() / PAGE_SIZE
}

fn system_ram_bytes() -> usize {
    // this seems to take 60 ms. :'-(
    sysinfo::System::new().get_total_memory() as usize * 1024
}
