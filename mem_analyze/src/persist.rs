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
use sys_info::hostname;
use byteorder::{ByteOrder, LittleEndian};

use rusoto_core::Region;
use rusoto_s3::S3Client;
use rusoto_s3::S3;
use rusoto_s3::PutObjectRequest;

pub fn write_process_memory(pid: i32, region: &str, memory: &super::ProcessMemory) -> std::io::Result<()> {
    let base_dir = format!("/tmp/wss/{}/{}", pid, memory.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    fs::create_dir_all(&base_dir)?;

    let hostname: String = match hostname() {
        Ok(hostname) => hostname,
        Err(e) => panic!(e),
    };

    for (segment_start, segment_data) in process_to_page_summary(&memory).into_iter() {
        write_to_file(&base_dir, segment_start, &segment_data)?;
        write_to_s3(region,
                    &format!("{}/{}", hostname, memory.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)),
                    segment_start, segment_data);
    }
    Ok(())
}

fn process_to_page_summary(memory: &super::ProcessMemory) -> HashMap<usize, Vec<u8>> {
    let mut segment_data = HashMap::new();
    for segment in &memory.segments {
        let mut page_summaries: Vec<u8>  = Vec::with_capacity(8 * segment.page_flags.len());
        page_summaries.resize(8 * segment.page_flags.len(), 0);
        LittleEndian::write_u64_into(&segment.page_flags, &mut page_summaries);
        segment_data.insert(segment.addr_start, page_summaries);
    }
    return segment_data;
}

fn write_to_file(base_dir: &str, segment_start: usize, segment_data: &Vec<u8>) -> std::io::Result<()> {
    let mut file = File::create(format!("{}/0x{:x}", base_dir, segment_start))?;
    info!("Persisted process memory metadata to: {:?}", file);
    file.write(&segment_data)?;
    Ok(())
}

fn write_to_s3(region_str: &str, base_key: &str, segment_start: usize, segment_data: Vec<u8>) {
    match S3Client::new(region_rusto(region_str)).put_object(PutObjectRequest {
        body: Some(segment_data.into()),
        bucket: format!("jgowans-wss-{}", region_str),
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

fn region_rusto(region_str: &str) -> Region {
    return match region_str {
        "us-east-1" => Region::UsEast1,
        "eu-west-2" => Region::EuWest2,
        "sa-east-1" => Region::SaEast1,
        _ => panic!("Invalid region: {}", region_str),
    };
}
