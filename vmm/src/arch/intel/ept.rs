use alloc::alloc::alloc;
use bitflags::bitflags;
use core::{
    alloc::Layout,
    ops::{Index, IndexMut},
    slice,
};
use x86_64::PhysAddr;

bitflags! {
    pub struct EptPointerFlags: u64 {
        const MEMORY_TYPE = 0b111;
        const PAGE_WALK_LENGTH = 0b111 << 3;
        const ACCESSED_AND_DIRTY = 1 << 6;
        const SUPERVISOR_SHADOW_STACK = 1 << 7;
    }
}

pub struct EptPointer(u64);

impl EptPointer {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn flags(&self) -> EptPointerFlags {
        EptPointerFlags::from_bits_truncate(self.0)
    }

    pub fn set_flags(&mut self, flags: EptPointerFlags) {
        self.0 = self.addr().as_u64() | flags.bits();
    }

    pub const fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000f_ffff_ffff_f000)
    }

    pub fn set_addr(&mut self, paddr: PhysAddr) {
        self.0 = paddr.as_u64() | self.flags().bits();
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[repr(u64)]
pub enum MemoryType {
    Uncacheable = 0,
    Writeback = 6,
}

pub struct EptTable(*mut u8);

impl EptTable {
    pub fn new() -> Self {
        let layout = Layout::from_size_align(4096, 4096).unwrap();
        let table = unsafe { alloc(layout) };

        Self(table)
    }

    pub fn zero(&mut self) {
        let table = unsafe { slice::from_raw_parts_mut(self.0, 4096) };
        table.fill(0);
    }
}

impl Index<usize> for EptTable {
    type Output = EptTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        let table = unsafe { slice::from_raw_parts(self.0 as *const EptTableEntry, 512) };
        &table[index]
    }
}

impl IndexMut<usize> for EptTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let table = unsafe { slice::from_raw_parts_mut(self.0 as *mut EptTableEntry, 512) };
        &mut table[index]
    }
}

#[repr(transparent)]
pub struct EptTableEntry(u64);

impl EptTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub const fn flags(&self) -> EptTableFlags {
        EptTableFlags::from_bits_truncate(self.0)
    }

    pub fn set_flags(&mut self, flags: EptTableFlags) {
        self.0 = self.addr().as_u64() | flags.bits();
    }

    pub const fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000f_ffff_ffff_f000)
    }

    pub fn set_addr(&mut self, paddr: PhysAddr) {
        self.0 = paddr.as_u64() | self.flags().bits();
    }
}

bitflags! {
    pub struct EptTableFlags: u64 {
        const READ_ACCESS = 1 << 0;
        const WRITE_ACCESS = 1 << 1;
        const EXECUTE_ACCESS = 1 << 2;
        const MEMORY_TYPE = 0b111 << 3;
        const IGNORE_PAT_MEMORY_TYPE = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const ACCESSED = 1 << 8;
        const DIRTY = 1 << 9;
        const EXECUTE_ACCESS_USER_MODE = 1 << 10;
        const VERIFY_GUEST_PAGING = 1 << 57;
        const PAGING_WRITE_ACCESS = 1 << 58;
        const SUPERVISOR_SHADOW_STACK = 1 << 60;
        const SUPPRESS_VE = 1 << 63;
    }
}
