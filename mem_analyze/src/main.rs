//extern crate data_encoding;
extern crate ring;
extern crate mem_analyze;

use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let pids: Vec<i32> = args[1..].iter().map(|p| p.parse().expect("Can't parse to i32")).collect();
    println!("PID supplied: {:?}\n", pids);
    loop {
        let process_memory = mem_analyze::dump::get_memory(pids[0])?;
        mem_analyze::persist::write_process_memory(pids[0], &process_memory)?;
        mem_analyze::statistics::page_analytics(&process_memory);
    }
}
