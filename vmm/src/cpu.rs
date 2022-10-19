use core::arch::asm;

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

pub struct CpuError {
    _kind: CpuErrorKind,
}

pub enum CpuErrorKind {
    _NotSupported,
}
