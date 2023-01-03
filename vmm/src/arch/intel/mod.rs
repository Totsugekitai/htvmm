mod ept;
mod vmcs;
mod vmexit_handlers;
pub mod vmx;

use crate::{
    arch::intel::vmx::VmExitGeneralPurposeRegister,
    cpu::{Cpu, CpuError},
    serial_println,
};
use core::arch::asm;
use crossbeam::atomic::AtomicCell;
use ept::{init_ept, EptPointer};
use lazy_static::lazy_static;
use vmcs::{VmcsField, VmcsRegion};
use vmx::{handle_vmexit, vmlaunch, vmxon, VmxError, VmxonRegion};

lazy_static! {
    pub static ref BSP: AtomicCell<IntelCpu> = AtomicCell::new(unsafe { IntelCpu::new() });
}

extern "C" {
    static vmexit_handler: u8;
}

pub struct IntelCpu {
    vmxon_region: VmxonRegion,
    vmcs_region: VmcsRegion,
    eptp: EptPointer,
}

impl IntelCpu {
    pub unsafe fn new() -> Self {
        Self {
            vmxon_region: VmxonRegion::new(),
            vmcs_region: VmcsRegion::new(),
            eptp: EptPointer::new(),
        }
    }

    fn is_vmx_supported() -> bool {
        let cpuid = Self::cpuid(1, 0);
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

        self.eptp = init_ept();
        self.vmcs_region
            .setup(self.eptp, unsafe { &vmexit_handler as *const u8 as u64 });
    }

    fn run_vm(&mut self) {
        unsafe {
            if let Err(e) = vmlaunch() {
                match e {
                    VmxError::InvalidPointer => panic!(),
                    VmxError::VmInstructionError => {
                        let error_code = self.vmcs_region.read(VmcsField::VmInstructionError);
                        asm!("mov r15, {}; hlt", in(reg) error_code, options(readonly, nostack, preserves_flags));
                    }
                }
            }
        }
    }
}

#[no_mangle]
unsafe fn resume_vm(gpr: *mut VmExitGeneralPurposeRegister) {
    let cpu = BSP.as_ptr().as_ref().unwrap();
    let exit_reason = cpu.vmcs_region.read(VmcsField::VmExitReason);
    let exit_qual = cpu.vmcs_region.read(VmcsField::ExitQualification);

    serial_println!("=== VMExit!!!!! ===");

    handle_vmexit(exit_reason, exit_qual, gpr);

    serial_println!("=== VMEntry!!!! ===");
}
