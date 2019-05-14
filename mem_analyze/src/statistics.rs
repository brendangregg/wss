use std::collections::HashMap;

struct PageAnalysisResult {
    pub total_pages: u64,
    pub zero_pages: u64,
    pub repeating_pages: u64,
    pub page_hash_counts: HashMap<String, u64>
}

pub fn page_analytics(memory: super::ProcessMemory) {
    for segment in memory.segments {
        println!("Segment start {:x} with size {}", segment.virtual_addr_start, segment.pages.len());
    }
    println!("You've analysed my memory...!");
}
