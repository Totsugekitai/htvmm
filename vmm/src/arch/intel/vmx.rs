use crate::{arch::intel::vmcs::VmcsRegion, BOOT_ARGS};
use alloc::alloc::alloc;
use core::{alloc::Layout, arch::asm, ptr, slice};
use x86_64::{
    registers::{
        control::{Cr4, Cr4Flags},
        model_specific::Msr,
    },
    PhysAddr,
};

pub unsafe fn vmxon(vmxon_region: &mut VmxonRegion) {
    let cr4 = Cr4::read();
    let cr4 = cr4 | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
    Cr4::write(cr4);

    let paddr = vmxon_region.paddr();
    if let Err(_e) = asm_vmxon(paddr) {
        panic!();
    }
}

unsafe fn asm_vmxon(phys_addr: PhysAddr) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmxon [rdi]; pushfq; pop rax", in("rdi") &phys_addr.as_u64(), out("rax") flags);
    check_vmx_error(flags)
}

pub unsafe fn vmclear(vmcs_region: &mut VmcsRegion) {
    let paddr = vmcs_region.paddr();
    if let Err(_e) = asm_vmclear(paddr) {
        panic!();
    }
}

unsafe fn asm_vmclear(phys_addr: PhysAddr) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmclear [rdi]; pushfq; pop rax", in("rdi") &phys_addr.as_u64(), out("rax") flags);
    check_vmx_error(flags)
}

pub unsafe fn vmptrld(vmcs_region: &VmcsRegion) {
    let paddr = vmcs_region.paddr();
    if let Err(_e) = asm_vmptrld(paddr) {
        panic!();
    }
}

unsafe fn asm_vmptrld(phys_addr: PhysAddr) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmptrld [rdi]; pushfq; pop rax", in("rdi") &phys_addr.as_u64(), out("rax") flags);
    check_vmx_error(flags)
}

fn check_vmx_error(rflags: u64) -> Result<(), VmxError> {
    let cf = (rflags & 0b1) == 1;
    let zf = ((rflags >> 6) & 0b1) == 1;

    if cf {
        Err(VmxError::InvalidPointer)
    } else if zf {
        Err(VmxError::VmInstructionError(0))
    } else {
        Ok(())
    }
}

pub struct VmxonRegion(*mut u8);

impl VmxonRegion {
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align_unchecked(4096, 4096);
        let region = alloc(layout);
        let region_slice = slice::from_raw_parts_mut(region, 4096);
        region_slice.fill(0);

        let ia32_vmx_basic = Msr::new(0x480).read(); // MSR_IA32_VMX_BASIC
        let vmcs_rev_id = (ia32_vmx_basic & 0x7fff_ffff) as u32;

        ptr::write_volatile(region as *mut u32, vmcs_rev_id);

        Self(region)
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0
    }

    fn paddr(&self) -> PhysAddr {
        let virt = i64::try_from(self.as_mut_ptr() as u64).unwrap();
        let phys = unsafe {
            u64::try_from(virt + BOOT_ARGS.as_ptr().as_ref().unwrap().vmm_phys_offset).unwrap()
        };
        PhysAddr::new(phys)
    }
}

pub enum VmxError {
    InvalidPointer,
    VmInstructionError(u8),
}
