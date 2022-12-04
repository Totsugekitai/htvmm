use crate::{arch::intel::vmcs::VmcsRegion, BOOT_ARGS};
use alloc::alloc::alloc;
use core::{alloc::Layout, arch::asm, ptr, slice};
use x86_64::{
    registers::{
        control::{Cr0, Cr0Flags, Cr4, Cr4Flags},
        model_specific::Msr,
    },
    PhysAddr,
};

use super::vmcs::VmcsField;

pub unsafe fn vmxon(vmxon_region: &mut VmxonRegion) -> Result<(), VmxError> {
    let cr4 = Cr4::read();
    let cr4_fixed_0 = Msr::new(0x488).read();
    let cr4_fixed_1 = Msr::new(0x489).read();
    let cr4 = cr4 & Cr4Flags::from_bits_unchecked(cr4_fixed_1);
    let cr4 = cr4 | Cr4Flags::from_bits_unchecked(cr4_fixed_0);
    Cr4::write(cr4);
    let cr4 = Cr4::read();
    let cr4 = cr4 | Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
    Cr4::write(cr4);

    let cr0 = Cr0::read();
    let cr0_fixed_0 = Msr::new(0x486).read();
    let cr0_fixed_1 = Msr::new(0x487).read();
    let cr0 = cr0 & Cr0Flags::from_bits_unchecked(cr0_fixed_1);
    let cr0 = cr0 | Cr0Flags::from_bits_unchecked(cr0_fixed_0);
    Cr0::write(cr0);

    let mut msr_ia32_feature_control = Msr::new(0x0000003a); // IA32_FEATURE_CONTROL
    let ia32_feature_control = unsafe { msr_ia32_feature_control.read() };
    let lock = ia32_feature_control & 0b1 == 1;
    if !lock {
        msr_ia32_feature_control.write(ia32_feature_control | 5);
    }

    let paddr = vmxon_region.paddr();
    asm_vmxon(paddr)
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

pub unsafe fn vmwrite(field: VmcsField, val: u32) {
    if let Err(_e) = asm_vmwrite(field, val) {
        panic!();
    }
}

pub unsafe fn vmwrite64(field: VmcsField, val: u64) {
    if let Err(_e) = asm_vmwrite64(field, val) {
        panic!();
    }
}

unsafe fn asm_vmwrite(field: VmcsField, val: u32) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmwrite edi, esi; pushfq; pop rax", in("rdi") field as u32, in("rsi") val, out("rax") flags);
    check_vmx_error(flags)
}

unsafe fn asm_vmwrite64(field: VmcsField, val: u64) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmwrite edi, esi; pushfq; pop rax", in("rdi") field as u32, in("rsi") val, out("rax") flags);
    if let Err(e) = check_vmx_error(flags) {
        return Err(e);
    }
    asm!("vmwrite edi, esi; pushfq; pop rax", in("rdi") field as u32 + 1, in("rsi") (val >> 32), out("rax") flags);
    check_vmx_error(flags)
}

fn check_vmx_error(flags: u64) -> Result<(), VmxError> {
    let cf = (flags & 0b1) == 1;
    let zf = ((flags >> 6) & 0b1) == 1;

    if cf {
        Err(VmxError::InvalidPointer)
    } else if zf {
        Err(VmxError::VmInstructionError(0)) // FIXME
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
        let virt = self.as_mut_ptr() as u64 as i64;
        let phys = (virt + BOOT_ARGS.load().vmm_phys_offset) as u64;
        PhysAddr::new(phys)
    }
}

pub enum VmxError {
    InvalidPointer,
    VmInstructionError(u8),
}
