mod vmcs;
mod vmx;

use crate::cpu::{Cpu, CpuError};
use vmcs::VmcsRegion;
use vmx::{vmxon, VmxonRegion};
use x86_64::registers::{
    control::{Cr4, Cr4Flags},
    model_specific::Msr,
};

pub struct IntelCpu {
    vmxon_region: VmxonRegion,
    vmcs_region: VmcsRegion,
}

impl IntelCpu {
    pub unsafe fn new() -> Self {
        Self {
            vmxon_region: VmxonRegion::new(),
            vmcs_region: VmcsRegion::new(),
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
        let lock = ia32_feature_control & 0b1 == 1;
        let vmx_enabled_inside_smx = (ia32_feature_control >> 1) & 0b1 == 1;
        let vmx_enabled_outside_smx = (ia32_feature_control >> 2) & 0b1 == 1;

        let smxe = Cr4::read().contains(Cr4Flags::SAFER_MODE_EXTENSIONS);

        lock && ((vmx_enabled_inside_smx && smxe) || (vmx_enabled_outside_smx && !smxe))
    }

    fn vmxon(&mut self) -> Result<(), CpuError> {
        unsafe {
            let vmxon = vmxon(&mut self.vmxon_region);
            if let Err(_e) = vmxon {
                Err(CpuError::VmxRelated)
            } else {
                Ok(())
            }
        }
    }
}

impl Cpu for IntelCpu {
    fn is_virtualization_supported(&self) -> bool {
        Self::is_vmx_supported() //& Self::is_vmxon_supported()
    }

    fn enable_virtualization(&mut self) -> Result<(), CpuError> {
        if !self.is_virtualization_supported() {
            Err(CpuError::NotSupported)
        } else {
            Self::vmxon(self)
        }
    }

    fn disable_virtualization(&mut self) -> Result<(), CpuError> {
        Ok(())
    }

    unsafe fn init_as_bsp(&mut self) {
        self.vmcs_region.clear();
        self.vmcs_region.load();
        // self.vmcs_region.setup();
    }
}
