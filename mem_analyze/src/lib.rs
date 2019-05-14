#[macro_use] extern crate scan_fmt;
extern crate byteorder;

pub mod statistics;
pub mod dump;

pub struct ProcessMemory {
    // virtual mem start to vector of page data
    pub segments: Vec<VirtualSegment>,
}


pub struct VirtualSegment {
    pub virtual_addr_start: usize,
    pub pages: Vec<Page>
}

pub struct Page {
    pub page_status: PageStatus,
    pub data: Vec<u8>,
}

pub enum PageStatus {
    Unmapped,
    Swapped,
    MappedIdle,
    MappedActive
}

