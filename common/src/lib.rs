#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PhysAddr(u64);

impl PhysAddr {
    pub fn new(pa: u64) -> Self {
        Self(pa)
    }

    pub fn to_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub fn new(va: u64) -> Self {
        Self(va)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BootArgs {
    pub file_size: u64,
    pub map_paddr: PhysAddr,
}
