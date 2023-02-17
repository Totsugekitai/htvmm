use crate::{
    arch::intel::{vmcs::VmcsField, vmx::VmExitGeneralPurposeRegister, BSP},
    cpu::guest_virt_to_guest_phys,
    serial_print, serial_println,
};
use alloc::string::String;
use iced_x86::{Decoder, DecoderOptions, Formatter, GasFormatter, Instruction};
use x86_64::PhysAddr;

pub fn cpuid(gpr: &mut VmExitGeneralPurposeRegister) {
    let eax = gpr.rax as u32;
    let ecx = gpr.rcx as u32;
    let cpuid = unsafe { core::arch::x86_64::__cpuid_count(eax, ecx) };
    gpr.rax = cpuid.eax as u64;
    gpr.rbx = cpuid.ebx as u64;
    gpr.rcx = cpuid.ecx as u64;
    gpr.rdx = cpuid.edx as u64;
}

pub fn cr_access(qual: u64, gpr: &mut VmExitGeneralPurposeRegister) {
    let bsp = unsafe { BSP.as_ptr().as_mut().unwrap() };
    let cr_number = qual & 0b1111;
    let access_type = (qual & 0b11_0000) >> 4;
    let _lmsw_operand_size = (qual & 0b100_0000) >> 6;
    let gpr_for_mov = (qual & 0b1111_0000_0000) >> 8;
    serial_println!("[CR{cr_number}] ACCESS TYPE: {access_type}, GPR: {gpr_for_mov}");

    match access_type {
        0 => {
            // mov to cr
            let value = match gpr_for_mov {
                0 => gpr.rax,
                1 => gpr.rcx,
                2 => gpr.rdx,
                3 => gpr.rbx,
                4 => bsp.vmcs_region.read(VmcsField::GuestRsp),
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
            match cr_number {
                0 => bsp.vmcs_region.write(VmcsField::GuestCr0, value),
                3 => bsp.vmcs_region.write(VmcsField::GuestCr3, value),
                4 => bsp.vmcs_region.write(VmcsField::GuestCr4, value),
                _ => panic!(),
            };
            serial_println!("CR{cr_number} write: 0x{value:016x}");
        }
        1 => {
            // mov from cr
            let value = match cr_number {
                0 => bsp.vmcs_region.read(VmcsField::GuestCr0),
                3 => bsp.vmcs_region.read(VmcsField::GuestCr3),
                4 => bsp.vmcs_region.read(VmcsField::GuestCr4),
                _ => panic!(),
            };
            match gpr_for_mov {
                0 => gpr.rax = value,
                1 => gpr.rcx = value,
                2 => gpr.rdx = value,
                3 => gpr.rbx = value,
                4 => bsp.vmcs_region.write(VmcsField::GuestRsp, value),
                5 => gpr.rbp = value,
                6 => gpr.rsi = value,
                7 => gpr.rdi = value,
                8 => gpr.r8 = value,
                9 => gpr.r9 = value,
                10 => gpr.r10 = value,
                11 => gpr.r11 = value,
                12 => gpr.r12 = value,
                13 => gpr.r13 = value,
                14 => gpr.r14 = value,
                15 => gpr.r15 = value,
                _ => panic!(),
            }
            serial_println!("CR{cr_number} read: 0x{value:016x}");
        }
        _ => panic!(),
    }
}

pub fn triple_fault() {
    let bsp = unsafe { BSP.as_ptr().as_ref().unwrap() };
    let guest_rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let guest_cr3 = bsp.vmcs_region.read(VmcsField::GuestCr3);
    let guest_rip_phys = guest_virt_to_guest_phys(guest_rip, guest_cr3);

    dump_instructions(guest_rip_phys, 0x20);

    x86_64::instructions::hlt();
}

pub fn ept_violation(_gpr: &mut VmExitGeneralPurposeRegister) {
    let bsp = unsafe { BSP.as_ptr().as_ref().unwrap() };
    let guest_rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let guest_cr3 = bsp.vmcs_region.read(VmcsField::GuestCr3);
    let guest_rip_phys = guest_virt_to_guest_phys(guest_rip, guest_cr3);

    dump_instructions(guest_rip_phys, 0x20);

    x86_64::instructions::hlt();
}

pub fn init_signal() {
    let bsp = unsafe { BSP.as_ptr().as_ref().unwrap() };
    let guest_rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let guest_cr3 = bsp.vmcs_region.read(VmcsField::GuestCr3);
    let guest_rip_phys = guest_virt_to_guest_phys(guest_rip, guest_cr3);

    dump_instructions(guest_rip_phys, 0x20);

    x86_64::instructions::hlt();
}

fn dump_instructions(guest_rip_phys: PhysAddr, len: usize) {
    let code = unsafe { core::slice::from_raw_parts(guest_rip_phys.as_u64() as *const u8, len) };
    let mut decoder = Decoder::with_ip(64, code, guest_rip_phys.as_u64(), DecoderOptions::NONE);
    let mut formatter = GasFormatter::new();
    let mut output = String::new();
    let mut instruction = Instruction::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        output.clear();
        formatter.format(&instruction, &mut output);
        serial_print!("{:016x} ", instruction.ip());
        let start_index = (instruction.ip() - guest_rip_phys.as_u64()) as usize;
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
}
