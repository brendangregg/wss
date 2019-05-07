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
    println!("Number of repeating pattern pages (excl. zero): {}", repeating_pattern_page_count - zero_page_count);
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

fn check_repeating_64_bit_pattern(data: &[u8]) -> bool {
    for idx in 8..data.len() {
        if data[idx] != data[idx-8] {
            return false;
        }
    }
    true
}
