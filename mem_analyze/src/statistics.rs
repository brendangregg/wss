pub fn page_analytics(memory: &super::ProcessMemory) {
    let mut total_pages = 0;
    let mut zero_pages = 0;
    let mut active_pages = 0;
    for segment in &memory.segments {
        for page_flags in &segment.page_flags {
            total_pages += 1;
            if page_flags & (1 << super::ZERO_PAGE_BIT) != 0 {
                zero_pages += 1;
            }
            if page_flags & (1 << super::ACTIVE_PAGE_BIT) != 0 {
                active_pages += 1;
            }
        }
        debug!("Segment start {:x} with size {}", segment.addr_start, segment.page_flags.len());
    }
    info!("Total pages: {}", total_pages);
    log_info("Zero pages", zero_pages, total_pages);
    log_info("Active pages", active_pages, total_pages);

    fn log_info(name: &str, val: u64, total: u64) {
        info!("{}", format!("{} pages: {} = {:.0}%", name, val, 100.0 * val as f32 / total as f32));
    }
}

