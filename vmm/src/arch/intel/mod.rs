mod vmx;

use crate::cpu::Cpu;
use core::arch::asm;
use x86_64::registers::control::{Cr4, Cr4Flags};

pub struct IntelCpu {
    vmxon_region: VmxonRegion,
}

impl IntelCpu {
    pub fn new() -> Self {
        Self {
            vmxon_region: VmxonRegion::new(),
        }
    }

    unsafe fn vmxon(&self) {
        let cr4 = Cr4::read();
        let cr4 = cr4 | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
        Cr4::write(cr4);

        let vmxon_region = self.vmxon_region.ptr_head();

        asm!("vmxon [rax]", in("rax") vmxon_region);
    }
}

impl Cpu for IntelCpu {
    fn is_virtualization_supported(&self) -> bool {
        let cpuid = Self::cpuid(1);
        let vmx = cpuid.ecx & (1 << 5);
        0 < vmx
    }

    fn enable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        unsafe {
            Self::vmxon(self);
        }
        Ok(())
    }

    fn disable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        Ok(())
    }
}

#[repr(C, align(4096))]
struct VmxonRegion {
    data: [u8; 4096],
}

impl VmxonRegion {
    fn new() -> Self {
        Self { data: [0; 4096] }
    }

    fn ptr_head(&self) -> u64 {
        &self.data as *const u8 as u64
    }
}
