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
use std::collections::HashMap;

use rusoto_core::Region;
use rusoto_s3::S3Client;
use rusoto_s3::S3;
use rusoto_s3::PutObjectRequest;

pub fn write_process_memory(pid: i32, memory: &super::ProcessMemory) -> std::io::Result<()> {
    let base_dir = format!("/tmp/wss/{}/{}", pid, memory.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    fs::create_dir_all(&base_dir)?;

    for (segment_start, segment_data) in process_to_page_summary(&memory).into_iter() {
        write_to_file(&base_dir, segment_start, &segment_data)?;
        write_to_s3(&format!("wss/{}", memory.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)),
                    segment_start, segment_data);
    }
    Ok(())
}

fn process_to_page_summary(memory: &super::ProcessMemory) -> HashMap<usize, Vec<u8>> {
    let mut segment_data = HashMap::new();
    for segment in &memory.segments {
        let mut page_summaries: Vec<u8>  = Vec::with_capacity(segment.pages.len());
        for page in &segment.pages {
            let mut page_summary;
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
        segment_data.insert(segment.virtual_addr_start, page_summaries);
    }
    return segment_data;
}

fn write_to_file(base_dir: &str, segment_start: usize, segment_data: &Vec<u8>) -> std::io::Result<()> {
    let mut file = File::create(format!("{}/0x{:x}", base_dir, segment_start))?;
    info!("Persisted process memory metadata to: {:?}", file);
    file.write(&segment_data)?;
    Ok(())
}

fn write_to_s3(base_key: &str, segment_start: usize, segment_data: Vec<u8>) {
    let s3client = S3Client::new(Region::EuWest2);
    match s3client.put_object(PutObjectRequest {
        body: Some(segment_data.into()),
        bucket: "jgowans".to_string(),
        key: format!("{}/0x{:x}", base_key, segment_start),
        ..Default::default()
    }).sync() {
        Ok(_resp) => {
            info!("PutObject success");
        },
        Err(error) => {
            error!("PutObject error: {:?}", error);
        }
    }
}
