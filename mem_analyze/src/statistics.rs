pub fn page_analytics(memory: &super::ProcessMemory) {
    let mut total_pages = 0;
    let mut zero_pages = 0;
    let mut unmapped_pages = 0;
    let mut swapped_pages = 0;
    let mut idle_pages = 0;
    let mut active_pages = 0;
    let mut repeating_64_bit_patterns = 0;
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
            if page.repeating_64_bit_pattern() {
                repeating_64_bit_patterns += 1;
            }
        }
        debug!("Segment start {:x} with size {}", segment.virtual_addr_start, segment.pages.len());
    }
    info!("Total pages: {}", total_pages);
    log_info("Unmapped", unmapped_pages, total_pages);
    log_info("Zero    ", zero_pages, total_pages);
    log_info("Active  ", active_pages, total_pages);
    log_info("Idle    ", idle_pages, total_pages);
    log_info("Swapped ", swapped_pages, total_pages);
    log_info("R 64bit ", repeating_64_bit_patterns, total_pages);

    fn log_info(name: &str, val: u64, total: u64) {
        info!("{}", format!("{} pages: {} = {:.0}%", name, val, 100.0 * val as f32 / total as f32));
    }
}

