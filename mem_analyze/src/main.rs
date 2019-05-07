//extern crate data_encoding;
extern crate ring;

use std::vec::Vec;
use std::env;
use std::fs::read;
use std::fs::read_dir;
use std::fs::DirEntry;
use std::hash::Hash;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

const PAGE_SIZE: usize = 4096;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let pids: Vec<i32> = args[1..].iter().map(|p| p.parse().expect("Can't parse to i32")).collect();
    println!("PID supplied: {:?}\n", pids);

    let mut total_pages = 0;
    let mut zero_page_count: u64 = 0;
    let mut repeating_pattern_page_count: u64 = 0;
    let mut page_content_counts = HashMap::new();

    //let path = format!("/tmp/raw-mem/{}/0x7f2d67a0c000:0:102400.mem", pid);
    //let path = format!("/tmp/raw-mem/{}/0x7f00e50cd000:57344:22125.mem", pids[0]);
    let mut paths: Vec<DirEntry> = Vec::new();
    for pid in pids {
        for dir_entry in read_dir(format!("/tmp/raw-mem/{}", pid))? {
            let dir_entry = dir_entry?;
            paths.push(dir_entry);
        }
    }
    for dir_entry in paths {
        let file_data = read(&dir_entry.path())?;
        let number_of_pages = file_data.len() / PAGE_SIZE;
        total_pages += number_of_pages;
        for page_number in  0..number_of_pages {
            let page_data = &file_data[(page_number * PAGE_SIZE)..((page_number + 1) * PAGE_SIZE)];
            if check_zero(&page_data) == true {
                zero_page_count += 1;
            }
            if check_repeating_64_bit_pattern(&page_data) {
                repeating_pattern_page_count += 1;
            }
            let mut hash = DefaultHasher::new();
            page_data.hash(&mut hash);
            *page_content_counts.entry(hash.finish()).or_insert(0) += 1;
        }
    }
    let mut page_content_counts_counts = HashMap::new();
    for occurances in page_content_counts.values() {
        *page_content_counts_counts.entry(occurances).or_insert(0) += 1;
    }
    println!("Total pages: {}", total_pages);
    println!("Number of zero pages: {}", zero_page_count);
    println!("Number of repeating pattern pages: {}", repeating_pattern_page_count);
    println!("Occurancs: {:?}", page_content_counts_counts);
    Ok(())
}

fn check_zero(data: &[u8]) -> bool {
    for &d in data {
        if d != 0 as u8 {
            return false;
        }
    }
    true
}

// There's probably a better way to do this but I'm a rust n00b.
fn check_repeating_64_bit_pattern(data: &[u8]) -> bool {
    let mut first_val = 0;
    for idx in 0..data.len() / 8 {
        let mut val = 0;
        val += (data[idx+0] as u64) <<  0;
        val += (data[idx+1] as u64) <<  8;
        val += (data[idx+2] as u64) << 16;
        val += (data[idx+3] as u64) << 24;
        val += (data[idx+4] as u64) << 32;
        val += (data[idx+5] as u64) << 40;
        val += (data[idx+6] as u64) << 48;
        val += (data[idx+7] as u64) << 56;
        
        if idx == 0 {
            first_val = val;
        } else {
            if val != first_val {
                return false;
            }
        }
    }
    true
}
