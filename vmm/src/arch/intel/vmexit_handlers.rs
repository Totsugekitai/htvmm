use crate::{
    arch::intel::{vmcs::VmcsField, vmx::VmExitGeneralPurposeRegister, BSP},
    serial_print, serial_println,
};
use alloc::string::String;
use iced_x86::{Decoder, DecoderOptions, Formatter, GasFormatter, Instruction};
use x86_64::{
    structures::paging::{PageTable, PageTableFlags},
    PhysAddr,
};

pub fn ept_violation(_gpr: &mut VmExitGeneralPurposeRegister) {
    let bsp = unsafe { BSP.as_ptr().as_ref().unwrap() };
    let guest_rip = bsp.vmcs_region.read(VmcsField::GuestRip);
    let guest_cr3 = bsp.vmcs_region.read(VmcsField::GuestCr3);
    let guest_rip_phys = guest_virt_to_guest_phys(guest_rip, guest_cr3);

    let code = unsafe { core::slice::from_raw_parts(guest_rip_phys.as_u64() as *const u8, 0x50) };
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
    x86_64::instructions::hlt();
}

fn guest_virt_to_guest_phys(guest_virt: u64, guest_cr3: u64) -> PhysAddr {
    let pml4_index = (guest_virt >> 39) as usize;
    let pdp_index = ((guest_virt >> 30) & 0b1_1111_1111) as usize;
    let pd_index = ((guest_virt >> 21) & 0b1_1111_1111) as usize;
    let pt_index = ((guest_virt >> 12) & 0b1_1111_1111) as usize;

    let guest_pml4_phys = PhysAddr::new(guest_cr3 & !0b1111_1111_1111);
    let guest_pml4 = unsafe {
        (guest_pml4_phys.as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pml4_entry = &guest_pml4[pml4_index];
    let guest_pdp = unsafe {
        (guest_pml4_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pdp_entry = &guest_pdp[pdp_index];
    if guest_pdp_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
        return guest_pdp_entry.addr();
    }

    let guest_pd = unsafe {
        (guest_pdp_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pd_entry = &guest_pd[pd_index];
    if guest_pd_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
        return guest_pd_entry.addr();
    }

    let guest_pt = unsafe {
        (guest_pd_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pt_entry = &guest_pt[pt_index];
    guest_pt_entry.addr()
}
