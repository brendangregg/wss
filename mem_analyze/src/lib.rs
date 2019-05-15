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

pub struct ProcessMemory {
    pub timestamp: DateTime<Utc>,
    // virtual mem start to vector of page data
    pub segments: Vec<VirtualSegment>,
}


pub struct VirtualSegment {
    pub virtual_addr_start: usize,
    pub pages: Vec<Page>
}

pub struct Page {
    pub status: PageStatus,
    pub data: Option<Vec<u8>>,
}

impl Page {
    pub fn is_zero(&self) -> bool {
        match self.data {
            None => return false,
            Some(ref data) => {
                for byte in data {
                    if *byte != 0 {
                        return false
                    }
                }
                return true
            }
        }
    }

    fn repeating_64_bit_pattern(&self) -> bool {
        match self.data {
            None => return false,
            Some(ref data) => {
                for idx in 8..data.len() {
                    if data[idx] != data[idx-8] {
                        return false;
                    }
                }
                return true
            }
        }
    }
}

#[derive(PartialEq)]
pub enum PageStatus {
    Unmapped,
    Swapped,
    Idle,
    Active
}

