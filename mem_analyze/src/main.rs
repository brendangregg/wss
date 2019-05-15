//extern crate data_encoding;
extern crate ring;
extern crate mem_analyze;
extern crate simplelog;

#[macro_use]
extern crate log;

use std::env;
use simplelog::*;
use chrono::Utc;

const SLEEP_TIME: u64 = 10;

fn main() -> std::io::Result<()> {

    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Debug, Config::default()).unwrap(),
        ]
    ).unwrap();

    let args: Vec<String> = env::args().collect();
    let pids: Vec<i32> = args[1..].iter().map(|p| p.parse().expect("Can't parse to i32")).collect();
    info!("PID supplied: {:?}\n", pids);
    loop {
        let start_time = Utc::now();
        let process_memory = mem_analyze::dump::get_memory(pids[0], SLEEP_TIME)?;
        mem_analyze::persist::write_process_memory(pids[0], &process_memory)?;
        mem_analyze::statistics::page_analytics(&process_memory);
        info!("---------- Completed analysis in in {} ms ----------",
              (Utc::now() - start_time).num_milliseconds());
    }
}
