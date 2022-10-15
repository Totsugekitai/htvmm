#![no_std]

use x86_64::{registers::control::Cr3Flags, PhysAddr};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BootArgs {
    pub uefi_cr3: PhysAddr,
    pub uefi_cr3_flags: Cr3Flags,
}
