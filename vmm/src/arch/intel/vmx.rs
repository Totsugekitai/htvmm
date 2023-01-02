use crate::{
    arch::intel::{
        vmcs::{VmcsField, VmcsRegion},
        IntelCpu, BSP,
    },
    cpu::Cpu,
    emu::decode_one,
    serial_print, serial_println, BOOT_ARGS,
};
use alloc::{alloc::alloc, string::String};
use common::constants;
use core::{alloc::Layout, arch::asm, ptr, slice};
use iced_x86::Formatter;
use x86_64::{
    registers::{
        control::{Cr0, Cr0Flags, Cr4, Cr4Flags},
        model_specific::Msr,
    },
    PhysAddr,
};

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

pub unsafe fn vmread(field: VmcsField) -> u64 {
    if let Ok(val) = asm_vmread(field) {
        val
    } else {
        panic!();
    }
}

unsafe fn asm_vmread(field: VmcsField) -> Result<u64, VmxError> {
    let mut flags;
    let mut value;
    asm!("vmread rdi, rsi; pushfq; pop rax", in("rsi") field as u32, out("rdi") value, out("rax") flags);
    if let Err(e) = check_vmx_error(flags) {
        Err(e)
    } else {
        Ok(value)
    }
}

pub unsafe fn vmwrite(field: VmcsField, value: u64) {
    if let Err(e) = asm_vmwrite(field, value) {
        panic!();
    }
}

unsafe fn asm_vmwrite(field: VmcsField, value: u64) -> Result<(), VmxError> {
    let mut flags;
    asm!("vmwrite rdi, rsi; pushfq; pop rax", in("rdi") field as u32, in("rsi") value, out("rax") flags);
    check_vmx_error(flags)
}

pub unsafe fn vmlaunch() -> Result<(), VmxError> {
    asm_vmlaunch()
}

unsafe fn asm_vmlaunch() -> Result<(), VmxError> {
    let mut flags;
    asm!("vmlaunch; pushfq; pop rax", out("rax") flags);
    check_vmx_error(flags)
}

fn check_vmx_error(flags: u64) -> Result<(), VmxError> {
    let cf = (flags & 0b1) == 1;
    let zf = ((flags >> 6) & 0b1) == 1;

    if cf {
        Err(VmxError::InvalidPointer)
    } else if zf {
        Err(VmxError::VmInstructionError)
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub struct VmxonRegion(*mut u8);

unsafe impl Send for VmxonRegion {}

impl VmxonRegion {
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align_unchecked(4096, 4096);
        let region = alloc(layout);
        let region_slice = slice::from_raw_parts_mut(region, 4096);
        region_slice.fill(0);

        let ia32_vmx_basic = Msr::new(constants::MSR_IA32_VMX_BASIC).read();
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
    VmInstructionError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum VmExitReason {
    ExceptionOrNmi = 0,
    ExternalInterrupt = 1,
    TripleFault = 2,
    InitSignal = 3,
    StartupIpi = 4,
    IoSmi = 5,
    OtherSmi = 6,
    InterruptWindow = 7,
    NmiWindow = 8,
    TaskSwitch = 9,
    Cpuid = 10,
    Getsec = 11,
    Hlt = 12,
    Invd = 13,
    Invlpg = 14,
    Rdpmc = 15,
    Rdtsc = 16,
    Rsm = 17,
    Vmcall = 18,
    Vmclear = 19,
    Vmlaunch = 20,
    Vmptrld = 21,
    Vmptrst = 22,
    Vmread = 23,
    Vmresume = 24,
    Vmwrite = 25,
    Vmxoff = 26,
    Vmxon = 27,
    CrAccess = 28,
    MovDr = 29,
    IoInstruction = 30,
    Rdmsr = 31,
    Wrmsr = 32,
    VmentryFailInvalidGuestState = 33,
    VmentryFailMsrLoading = 34,
    Mwait = 36,
    MonitorTrapFlag = 37,
    Monitor = 39,
    Pause = 40,
    VmentryFailMachineCheckEvent = 41,
    TprBelowThreshold = 43,
    ApicAccess = 44,
    VirtualizedEoi = 45,
    AccessGdtrOrIdtr = 46,
    AccessLdtrOrTr = 47,
    EptViolation = 48,
    EptMisconfiguration = 49,
    Invept = 50,
    Rdtscp = 51,
    VmxPreemptionTimerExpired = 52,
    Invvpid = 53,
    WbinvdOrWbnoinvd = 54,
    Xsetbv = 55,
    ApicWrite = 56,
    Rdrand = 57,
    Invpcid = 58,
    Vmfunc = 59,
    Encls = 60,
    Rdseed = 61,
    PageModificationLogFull = 62,
    Xsaves = 63,
    Xrstors = 64,
    Pconfig = 65,
    SppRelatedEvent = 66,
    Umwait = 67,
    Tpause = 68,
    Loadiwkey = 69,
}

#[derive(Debug)]
#[repr(C)]
pub struct VmExitGeneralPurposeRegister {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rbp: u64,
}

pub fn handle_vmexit(reason: u64, qual: u64, gpr: *mut VmExitGeneralPurposeRegister) {
    let reason = unsafe { core::mem::transmute(reason) };
    serial_println!("{reason:?}, qualification: 0x{qual:x}");
    let cpu = unsafe { BSP.as_ptr().as_ref().unwrap() };
    let gpr = unsafe { gpr.as_mut().unwrap() };
    // let gpr_const = unsafe {
    //     (gpr as *const VmExitGeneralPurposeRegister)
    //         .as_ref()
    //         .unwrap()
    // };
    let rsp = cpu.vmcs_region.read(VmcsField::GuestRsp);
    let rip = cpu.vmcs_region.read(VmcsField::GuestRip);
    let rflags = cpu.vmcs_region.read(VmcsField::GuestRflags);
    serial_println!(
        "rax: 0x{:016x} rbx: 0x{:016x} rcx: 0x{:016x} rdx: 0x{:016x}",
        gpr.rax,
        gpr.rbx,
        gpr.rcx,
        gpr.rdx
    );
    serial_println!(
        "rsi: 0x{:016x} rdi: 0x{:016x} rsp: 0x{:016x} rbp: 0x{:016x}",
        gpr.rsi,
        gpr.rdi,
        rsp,
        gpr.rbp
    );
    serial_println!(
        " r8: 0x{:016x}  r9: 0x{:016x} r10: 0x{:016x} r11: 0x{:016x}",
        gpr.r8,
        gpr.r9,
        gpr.r10,
        gpr.r11
    );
    serial_println!(
        "r12: 0x{:016x} r13: 0x{:016x} r14: 0x{:016x} r15: 0x{:016x}",
        gpr.r12,
        gpr.r13,
        gpr.r14,
        gpr.r15
    );
    serial_println!("rip: 0x{:016x} rflags: 0x{:016x}", rip, rflags);

    match reason {
        VmExitReason::TripleFault => {
            x86_64::instructions::hlt();
        }
        VmExitReason::EptViolation => {
            use iced_x86::{Decoder, DecoderOptions, GasFormatter, Instruction};
            let code = unsafe { core::slice::from_raw_parts(rip as *const u8, 0x50) };
            let mut decoder = Decoder::with_ip(64, code, rip, DecoderOptions::NONE);
            let mut formatter = GasFormatter::new();
            let mut output = String::new();
            let mut instruction = Instruction::default();
            while decoder.can_decode() {
                decoder.decode_out(&mut instruction);
                output.clear();
                formatter.format(&instruction, &mut output);
                serial_print!("{:016x} ", instruction.ip());
                let start_index = (instruction.ip() - rip) as usize;
                let instr_bytes = &code[start_index..(start_index + instruction.len())];
                for b in instr_bytes.iter() {
                    serial_print!("{:02x} ", b);
                }
                if instr_bytes.len() < 10 {
                    for _ in 0..(10 - instr_bytes.len()) {
                        serial_print!("   ");
                    }
                }
                serial_println!(" {output}");
            }
            x86_64::instructions::hlt();
        }
        VmExitReason::CrAccess => {
            let cr3 = cpu.vmcs_region.read(VmcsField::GuestCr3);
            serial_println!("before CR3: {cr3:x}");
            handle_cr_access(qual, gpr);
        }
        VmExitReason::ExceptionOrNmi => {
            let vmexit_intr_info = cpu.vmcs_region.read(VmcsField::VmExitIntrInfo);
            let vector = vmexit_intr_info & 0b11111111;
            let intr_type = (vmexit_intr_info & 0b11100000000) >> 8;
            let error_code_valid = ((vmexit_intr_info & 0b100000000000) >> 11) == 1;
            serial_println!("Interruption vector: {vector}");
            serial_println!("Interruption type number: {intr_type}");
            if error_code_valid {
                let error_code = cpu.vmcs_region.read(VmcsField::VmExitIntrErrorCode);
                serial_println!("Error code: {error_code}");
            }
        }
        VmExitReason::Cpuid => handle_cpuid(gpr),
        _ => x86_64::instructions::hlt(),
    }
}

fn handle_cr_access(qual: u64, gpr: *mut VmExitGeneralPurposeRegister) {
    let cr_number = qual & 0b1111;
    let access_type = (qual & 0b110000) >> 4;
    let lmsw_operand_size = (qual & 0b1000000) >> 6;
    let gpr_for_mov = (qual & 0b111100000000) >> 8;
    serial_println!("[CR{cr_number}] ACCESS TYPE: {access_type}, GPR: {gpr_for_mov}");
    match cr_number {
        0 => panic!(), //handle_cr0_access(access_type, lmsw_operand_size, gpr_for_mov),
        3 => handle_cr3_access(access_type, gpr_for_mov, gpr),
        4 => panic!(), // handle_cr4_access(access_type, gpr_for_mov),
        _ => panic!(),
    }
}

fn handle_cr0_access(access_type: u64, lmsw_operand_size: u64, gpr_for_mov: u64) {}

fn handle_cr3_access(access_type: u64, gpr_for_mov: u64, gpr: *mut VmExitGeneralPurposeRegister) {
    let gpr = unsafe { gpr.as_mut().unwrap() };
    let value = match gpr_for_mov {
        0 => gpr.rax,
        1 => gpr.rcx,
        2 => gpr.rdx,
        3 => gpr.rbx,
        4 => unsafe {
            BSP.as_ptr()
                .as_ref()
                .unwrap()
                .vmcs_region
                .read(VmcsField::GuestRsp)
        },
        5 => gpr.rbp,
        6 => gpr.rsi,
        7 => gpr.rdi,
        8 => gpr.r8,
        9 => gpr.r9,
        10 => gpr.r10,
        11 => gpr.r11,
        12 => gpr.r12,
        13 => gpr.r13,
        14 => gpr.r14,
        15 => gpr.r15,
        _ => panic!(),
    };
    match access_type {
        0 => handle_mov_to_cr3(value),
        1 => handle_mov_from_cr3(gpr_for_mov, gpr),
        _ => panic!(),
    }
}

fn handle_cr4_access(access_type: u64, gpr_for_mov: u64) {}

fn handle_mov_to_cr3(value: u64) {
    let bsp = unsafe { BSP.as_ptr().as_mut().unwrap() };
    bsp.vmcs_region.write(VmcsField::GuestCr3, value);
    serial_println!("after CR3: {value:x}");
    let rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let instruction = decode_one(rip);
    if instruction.is_err() {
        serial_println!("decode error");
        panic!();
    }
    let instruction = instruction.unwrap();
    serial_println!("instruction len: {}", instruction.len());
    bsp.vmcs_region
        .write(VmcsField::GuestRip, rip + instruction.len() as u64);
}

fn handle_mov_from_cr3(gpr_for_mov: u64, gpr: &mut VmExitGeneralPurposeRegister) {
    let bsp = unsafe { BSP.as_ptr().as_mut().unwrap() };
    let cr3 = bsp.vmcs_region.read(VmcsField::GuestCr3);
    match gpr_for_mov {
        0 => gpr.rax = cr3,
        1 => gpr.rcx = cr3,
        2 => gpr.rdx = cr3,
        3 => gpr.rbx = cr3,
        4 => bsp.vmcs_region.write(VmcsField::GuestRsp, cr3),
        5 => gpr.rbp = cr3,
        6 => gpr.rsi = cr3,
        7 => gpr.rdi = cr3,
        8 => gpr.r8 = cr3,
        9 => gpr.r9 = cr3,
        10 => gpr.r10 = cr3,
        11 => gpr.r11 = cr3,
        12 => gpr.r12 = cr3,
        13 => gpr.r13 = cr3,
        14 => gpr.r14 = cr3,
        15 => gpr.r15 = cr3,
        _ => panic!(),
    };
    let rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let instruction = decode_one(rip);
    if instruction.is_err() {
        serial_println!("decode error");
        panic!();
    }
    let instruction = instruction.unwrap();
    serial_println!("instruction len: {}", instruction.len());
    bsp.vmcs_region
        .write(VmcsField::GuestRip, rip + instruction.len() as u64);
}

fn handle_cpuid(gpr: &mut VmExitGeneralPurposeRegister) {
    let rax = gpr.rax;
    let cpuid = IntelCpu::cpuid(rax as u32);
    gpr.rax = (rax & !0xffff_ffff) | cpuid.eax as u64;
    gpr.rbx = (gpr.rbx & !0xffff_ffff) | cpuid.ebx as u64;
    gpr.rcx = (gpr.rcx & !0xffff_ffff) | cpuid.ecx as u64;
    gpr.rdx = (gpr.rdx & !0xffff_ffff) | cpuid.edx as u64;

    let bsp = unsafe { BSP.as_ptr().as_mut().unwrap() };
    let rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    bsp.vmcs_region.write(VmcsField::GuestRip, rip + 2);
}
