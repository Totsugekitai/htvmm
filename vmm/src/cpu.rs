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

pub enum CpuError {
    NotSupported,
    VmxRelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tr(u64);

impl Tr {
    pub fn get_reg() -> Self {
        let mut tr: u64;
        unsafe {
            asm!("sub rsp, 8; str [rsp]; pop {}", out(reg) tr);
        }
        Self(tr)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gdtr {
    limit: u16,
    base: u64,
}

impl Gdtr {
    pub fn get_reg() -> Self {
        let mut base: u64;
        let mut limit: u16;
        unsafe {
            asm!("sub rsp, 16; sgdt [rsp]; mov [rsp], dx; mov [rsp + 2], rcx; add rsp, 16", out("dx") limit, out("rcx") base);
        }
        Self { limit, base }
    }

    pub fn base(&self) -> u64 {
        self.base
    }

    pub fn limit(&self) -> u16 {
        self.limit
    }
}
