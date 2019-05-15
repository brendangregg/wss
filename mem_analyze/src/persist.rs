// Format of identifying byte:
// 0,1 : unmapped, swapped, idle, active
// 2 : zero
// 3 : repeating pattern
// 4 : hash follows
// 5 : ksm merged
// 6 : modified since last run
// 7 : version

use std::fs;
use std::fs::File;
use std::io::Write;
use chrono::SecondsFormat;

pub fn write_process_memory(pid: i32, memory: &super::ProcessMemory) -> std::io::Result<()> {
    let base_dir = format!("/tmp/wss/{}/{}", pid, memory.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    fs::create_dir_all(&base_dir);

    for segment in &memory.segments {
        let mut page_summaries: Vec<u8>  = Vec::with_capacity(segment.pages.len());
        for page in &segment.pages {
            let mut page_summary = 0;
            match page.status {
                super::PageStatus::Unmapped => page_summary = 0,
                super::PageStatus::Swapped => page_summary = 1,
                super::PageStatus::Idle => page_summary = 2,
                super::PageStatus::Active => page_summary = 3
            }
            if page.is_zero() {
                page_summary += 1 << 2;
            }
            page_summaries.push(page_summary);
        }
        let mut file = File::create(format!("{}/0x{:x}", base_dir, segment.virtual_addr_start))?;
        println!("Opened file: {:?}", file);
        file.write(&page_summaries)?;
    }
    Ok(())
}
