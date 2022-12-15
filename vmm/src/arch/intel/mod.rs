mod vmcs;
mod vmx;

use crate::cpu::{Cpu, CpuError};
use vmcs::{VmcsField, VmcsRegion};
use vmx::{vmlaunch, vmxon, VmxError, VmxonRegion};

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
        Self::is_vmx_supported()
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

    fn init_as_bsp(&mut self) {
        self.vmcs_region.clear();
        self.vmcs_region.load();
        self.vmcs_region.setup();
    }

    fn run_vm(&mut self) {
        unsafe {
            if let Err(e) = vmlaunch() {
                match e {
                    VmxError::InvalidPointer => panic!(),
                    VmxError::VmInstructionError => {
                        let error_code = self.vmcs_region.read(VmcsField::VmInstructionError);
                        use core::arch::asm;
                        asm!("mov r15, {}; hlt", in(reg) error_code, options(readonly, nostack, preserves_flags));
                    }
                }
            }
        }
    }
}
