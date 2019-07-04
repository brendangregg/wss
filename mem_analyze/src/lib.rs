#[macro_use]
extern crate scan_fmt;

extern crate byteorder;
extern crate chrono;

#[macro_use]
extern crate log;

pub mod statistics;
pub mod dump;
pub mod persist;

use chrono::{DateTime, Utc};

// https://www.kernel.org/doc/Documentation/vm/pagemap.txt
// We're going to steal bits from the PFN (0-54) of the /proc/pid/pagemap,
// while using the same bits of /proc/kpageflags
pub const ZERO_PAGE_BIT: u8 = 24;
pub const ACTIVE_PAGE_BIT: u8 = 27;

pub struct ProcessMemory {
    pub timestamp: DateTime<Utc>,
    // virtual mem start to vector of page data
    pub segments: Vec<Segment>,
}


pub struct Segment {
    pub addr_start: usize,
    // For now these flags are just what we get back from /proc/pid/pagemap
    // OR /proc/kpageflags. We may want to standardize bits at some point...
    pub page_flags: Vec<u64>
}
