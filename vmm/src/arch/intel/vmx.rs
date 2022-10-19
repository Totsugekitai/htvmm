use super::vmcs::VmcsRegion;
use alloc::alloc::alloc;
use core::{alloc::Layout, arch::asm};
use x86_64::registers::control::{Cr4, Cr4Flags};

pub unsafe fn vmxon(vmxon_region: &mut VmxonRegion) {
    let cr4 = Cr4::read();
    let cr4 = cr4 | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
    Cr4::write(cr4);

    asm!("vmxon [rax]", in("rax") vmxon_region.as_mut_ptr());
}

pub unsafe fn vmclear(vmcs_region: &mut VmcsRegion) {
    asm!("vmclear [rax]", in("rax") vmcs_region.as_mut_ptr());
}

pub struct VmxonRegion(*mut u8);

impl VmxonRegion {
    #[inline]
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align_unchecked(4096, 4096);
        Self(alloc(layout))
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0
    }
}
