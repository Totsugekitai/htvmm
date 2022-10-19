use core::arch::asm;
use x86_64::registers::control::{Cr4, Cr4Flags};

pub unsafe fn vmxon(vmxon_region: &VmxonRegion) {
    let cr4 = Cr4::read();
    let cr4 = cr4 | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
    Cr4::write(cr4);

    let vmxon_region = vmxon_region.as_u8_ptr() as u64;

    asm!("vmxon [rax]", in("rax") vmxon_region);
}

#[repr(align(4096))]
pub struct VmxonRegion([u8; 4096]);

impl VmxonRegion {
    pub const fn new() -> Self {
        Self([0; 4096])
    }

    pub fn as_u8_ptr(&self) -> *const u8 {
        &self.0[0] as *const u8
    }
}
