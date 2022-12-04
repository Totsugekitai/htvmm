use core::arch::asm;
use x86_64::{
    instructions::tables::sgdt, registers::segmentation::Segment, structures::gdt::SegmentSelector,
};

#[derive(Debug, Clone)]
pub struct Cpuid {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl Cpuid {
    pub fn new() -> Self {
        Self {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        }
    }
}

pub trait Cpu {
    fn cpuid(eax: u32) -> Cpuid {
        let mut cpuid = Cpuid::new();
        unsafe {
            asm!("cpuid","mov rdi, rbx",  inlateout("eax") eax => cpuid.eax, out("rdi") cpuid.ebx, out("ecx") cpuid.ecx, out("edx") cpuid.edx);
        }
        cpuid
    }
    fn is_virtualization_supported(&self) -> bool;
    fn enable_virtualization(&mut self) -> Result<(), CpuError>;
    fn disable_virtualization(&mut self) -> Result<(), CpuError>;
    unsafe fn init_as_bsp(&mut self);
}

pub enum CpuError {
    NotSupported,
    VmxRelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ldtr;

impl Segment for Ldtr {
    fn get_reg() -> SegmentSelector {
        let mut ldtr: u16 = 0;
        unsafe {
            asm!("sldt [{}]", in(reg) &mut ldtr, options(nostack, preserves_flags));
        }
        SegmentSelector(ldtr)
    }

    unsafe fn set_reg(sel: SegmentSelector) {
        asm!("lldt [{}]", in(reg) &sel.0, options(readonly, nostack, preserves_flags));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tr;

impl Segment for Tr {
    fn get_reg() -> SegmentSelector {
        let mut tr: u16 = 0;
        unsafe {
            asm!("str [{}]", in(reg) &mut tr, options(nostack, preserves_flags));
        }
        SegmentSelector(tr)
    }

    unsafe fn set_reg(sel: SegmentSelector) {
        asm!("ltr [{}]", in(reg) &sel.0, options(readonly, nostack, preserves_flags));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentDescriptor;

impl SegmentDescriptor {
    pub fn base(sel: &SegmentSelector) -> u32 {
        let i = sel.index() as usize;
        let gdt = sgdt();
        let gdt_limit = gdt.limit;
        let gdt = unsafe {
            core::slice::from_raw_parts(gdt.base.as_ptr() as *const u64, gdt_limit as usize)
        };
        let segment_descriptor = gdt[i];

        ((segment_descriptor & (0xff << (32 + 24))) as u32)
            | ((segment_descriptor & (0xff << 32)) as u32)
            | ((segment_descriptor & (0xffff << 16)) as u32)
    }

    pub fn limit(sel: &SegmentSelector) -> u32 {
        let mut limit: u32 = 0;
        unsafe {
            asm!("lsl {}, [{}]", in(reg) &mut limit, in(reg) &sel.0, options(nostack, preserves_flags));
        }
        limit
    }
}
