mod vmx;

use crate::cpu::Cpu;
use core::arch::asm;
use x86_64::registers::{
    control::{Cr4, Cr4Flags},
    model_specific::Msr,
};

pub struct IntelCpu {
    vmxon_region: &'static VmxonRegion,
}

impl IntelCpu {
    pub unsafe fn new() -> Self {
        Self {
            vmxon_region: &VMXON_REGION,
        }
    }

    fn is_vmx_supported() -> bool {
        let cpuid = Self::cpuid(1);
        let vmx = cpuid.ecx & (1 << 5);
        0 < vmx
    }

    fn is_vmxon_supported() -> bool {
        let msr_ia32_feature_control = Msr::new(0x0000003a); // IA32_FEATURE_CONTROL
        let ia32_feature_control = unsafe { msr_ia32_feature_control.read() };
        let lock = ia32_feature_control & 0b1;
        let vmxon_in_smx = (ia32_feature_control >> 1) & 0b1;
        let vmxon_outside_smx = (ia32_feature_control >> 2) & 0b1;
        (lock & vmxon_in_smx & vmxon_outside_smx) == 1
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
        Self::is_vmx_supported() & Self::is_vmxon_supported()
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

#[repr(align(4096))]
struct VmxonRegion([u8; 4096]);

impl VmxonRegion {
    const fn new() -> Self {
        Self([0; 4096])
    }

    fn ptr_head(&self) -> u64 {
        &self.0[0] as *const u8 as u64
    }
}

const VMXON_REGION: VmxonRegion = VmxonRegion::new();
