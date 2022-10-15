mod vmx;

use crate::cpu::Cpu;
use core::arch::asm;

const VMXON_REGION: VmxonRegion = VmxonRegion::new();
pub struct IntelCpu {
    vmxon_region: &'static VmxonRegion,
}

impl IntelCpu {
    pub fn new() -> Self {
        Self {
            vmxon_region: &VMXON_REGION,
        }
    }

    fn vmxon(&self) {
        let vmxon_region = self.vmxon_region as *const VmxonRegion;
        unsafe {
            asm!("vmxon [rax]", in("rax") vmxon_region);
        }
    }
}

impl Cpu for IntelCpu {
    fn is_virtualization_supported(&self) -> bool {
        let cpuid = Self::cpuid(1);
        let vmx = cpuid.ecx & (1 << 5);
        0 < vmx
    }

    fn enable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        Self::vmxon(self);
        Ok(())
    }

    fn disable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        Ok(())
    }
}

#[repr(align(4096))]
struct VmxonRegion {
    _data: [u8; 4096],
}

impl VmxonRegion {
    const fn new() -> Self {
        Self { _data: [0; 4096] }
    }
}
