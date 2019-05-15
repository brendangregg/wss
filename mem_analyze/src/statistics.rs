use std::collections::HashMap;

struct PageAnalysisResult {
    pub total_pages: u64,
    pub active: u64,
    pub zero_and_idle: u64,
    pub zero_and_active: u64,
    pub repeating_pages: u64,
    pub page_hash_counts: HashMap<String, u64>
}

pub fn page_analytics(memory: &super::ProcessMemory) {
    let mut total_pages = 0;
    let mut zero_pages = 0;
    let mut repeating_pages = 0;
    let mut unmapped_pages = 0;
    let mut swapped_pages = 0;
    let mut idle_pages = 0;
    let mut active_pages = 0;
    for segment in &memory.segments {
        for page in &segment.pages {
            total_pages += 1;
            match page.status {
                super::PageStatus::Unmapped => unmapped_pages += 1,
                super::PageStatus::Swapped => swapped_pages += 1,
                super::PageStatus::Idle => idle_pages += 1,
                super::PageStatus::Active => active_pages += 1,
            }
            if page.is_zero() {
                zero_pages += 1;
            }

        }
        println!("Segment start {:x} with size {}", segment.virtual_addr_start, segment.pages.len());
    }
    println!("Total pages: {}", total_pages);
    println!("Unmapped pages: {}", unmapped_pages);
    println!("Zero pages: {}", zero_pages);
}
