#[macro_use] extern crate scan_fmt;
extern crate byteorder;
extern crate chrono;

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
    pub data: Vec<u8>,
}

impl Page {
    pub fn is_zero(&self) -> bool {
        if self.data.len() == 0 {
            return false
        }
        for byte in &self.data {
            if *byte != 0 {
                return false
            }
        }
        true
    }

    fn repeating_64_bit_pattern(&self) -> bool {
        for idx in 8..self.data.len() {
            if self.data[idx] != self.data[idx-8] {
                return false;
            }
        }
        true
    }
}

#[derive(PartialEq)]
pub enum PageStatus {
    Unmapped,
    Swapped,
    Idle,
    Active
}

