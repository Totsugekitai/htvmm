#![no_std]

use x86_64::{registers::control::Cr3Flags, PhysAddr};

pub const VMM_AREA_SIZE: u64 = 512 * 1024 * 1024;
pub const VMM_AREA_HEAD_VADDR: usize = 0x1_0000_0000;
pub const VMM_HEAP_HEAD_VADDR: usize = VMM_AREA_HEAD_VADDR + (128 * 1024 * 1024);

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BootArgs {
    pub uefi_cr3: PhysAddr,
    pub uefi_cr3_flags: Cr3Flags,
}
