use crate::BOOT_ARGS;
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
        const MEMORY_TYPE_UNCACHEABLE = 0;
        const MEMORY_TYPE_WRITEBACK = 6;
        const PAGE_WALK_LENGTH_4 = 0b011 << 3;
        const ACCESSED_AND_DIRTY = 1 << 6;
        const SUPERVISOR_SHADOW_STACK = 1 << 7;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug)]
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

    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.0
    }

    pub fn paddr(&self) -> PhysAddr {
        let virt = i64::try_from(self.as_mut_ptr() as u64).unwrap();
        let phys = unsafe {
            u64::try_from(virt + BOOT_ARGS.as_ptr().as_ref().unwrap().vmm_phys_offset).unwrap()
        };
        PhysAddr::new(phys)
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

#[derive(Debug)]
#[repr(C)]
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
        const MEMORY_TYPE_UC = 0 << 3;
        const MEMORY_TYPE_WC = 1 << 3;
        const MEMORY_TYPE_WT = 4 << 3;
        const MEMORY_TYPE_WP = 5 << 3;
        const MEMORY_TYPE_WB = 6 << 3;
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

pub fn init_ept() -> EptPointer {
    let memory_size = unsafe { BOOT_ARGS.as_ptr().as_ref().unwrap().memory_size };
    let memory_size_gb = memory_size / (1024 * 1024 * 1024);

    let mut ept_pml4 = EptTable::new();
    ept_pml4.zero();

    let mut ept_pdpt = EptTable::new();
    ept_pdpt.zero();

    ept_pml4[0].set_addr(ept_pdpt.paddr());
    ept_pml4[0].set_flags(
        EptTableFlags::READ_ACCESS | EptTableFlags::WRITE_ACCESS | EptTableFlags::EXECUTE_ACCESS,
    );

    for i_pdpt in 0..(memory_size_gb as usize) {
        let mut ept_pdt = EptTable::new();

        ept_pdpt[i_pdpt].set_addr(ept_pdt.paddr());
        ept_pdpt[i_pdpt].set_flags(
            EptTableFlags::READ_ACCESS
                | EptTableFlags::WRITE_ACCESS
                | EptTableFlags::EXECUTE_ACCESS,
        );

        for i_pdt in 0..512 {
            ept_pdt[i_pdt].set_addr(PhysAddr::new(
                (2 * 1024 * 1024 * i_pdt as u64) + (1024 * 1024 * 1024 * i_pdpt as u64),
            ));
            ept_pdt[i_pdt].set_flags(
                EptTableFlags::HUGE_PAGE
                    | EptTableFlags::READ_ACCESS
                    | EptTableFlags::WRITE_ACCESS
                    | EptTableFlags::EXECUTE_ACCESS
                    | EptTableFlags::MEMORY_TYPE_WB,
            );
        }

        // if i_pdpt == 0 {
        //     ept_pdt[0].set_flags(EptTableFlags::HUGE_PAGE | EptTableFlags::MEMORY_TYPE_WB);
        // }
    }

    let mut eptp = EptPointer::new();
    eptp.set_addr(ept_pml4.paddr());
    eptp.set_flags(EptPointerFlags::MEMORY_TYPE_WRITEBACK | EptPointerFlags::PAGE_WALK_LENGTH_4);

    eptp
}
