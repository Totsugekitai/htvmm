mod vmx;

use crate::cpu::Cpu;
use x86_64::registers::model_specific::Msr;

pub struct IntelCpu {
    vmxon_region: &'static vmx::VmxonRegion,
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

    fn vmxon(&self) {
        unsafe {
            vmx::vmxon(self.vmxon_region);
        }
    }
}

impl Cpu for IntelCpu {
    fn is_virtualization_supported(&self) -> bool {
        Self::is_vmx_supported() & Self::is_vmxon_supported()
    }

    fn enable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        Self::vmxon(self);
        Ok(())
    }

    fn disable_virtualization(&mut self) -> Result<(), crate::cpu::CpuError> {
        Ok(())
    }
}

const VMXON_REGION: vmx::VmxonRegion = vmx::VmxonRegion::new();
