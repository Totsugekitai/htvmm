#![no_std]

pub mod constants;

use x86_64::{registers::control::Cr3Flags, PhysAddr};

pub const VMM_AREA_SIZE: u64 = 512 * 1024 * 1024;
pub const VMM_AREA_HEAD_VADDR: usize = 0x1_0000_0000;
pub const VMM_HEAP_HEAD_VADDR: usize = VMM_AREA_HEAD_VADDR + (128 * 1024 * 1024);
pub const VMM_HEAP_SIZE: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootArgs {
    pub uefi_cr3: PhysAddr,
    pub uefi_cr3_flags: Cr3Flags,
    pub vmm_phys_offset: i64,
    pub memory_size: u64,
    pub uefi_write_char: u64,
    pub uefi_output: u64,
}

impl BootArgs {
    pub const fn new() -> Self {
        Self {
            uefi_cr3: PhysAddr::new(0),
            uefi_cr3_flags: Cr3Flags::empty(),
            vmm_phys_offset: 0,
            memory_size: 0,
            uefi_write_char: 0,
            uefi_output: 0,
        }
    }
}
