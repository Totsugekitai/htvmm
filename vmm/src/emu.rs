// use crate::arch::intel::vmx::VmExitGeneralPurposeRegister;
use iced_x86::{Decoder, DecoderOptions, Instruction};

pub fn decode_one(rip: u64) -> Result<Instruction, ()> {
    let code = unsafe { core::slice::from_raw_parts(rip as *const u8, 10) };
    let mut decoder = Decoder::with_ip(64, code, rip, DecoderOptions::NONE);
    if decoder.can_decode() {
        Ok(decoder.decode())
    } else {
        Err(())
    }
}

// pub fn emulation_one(rip: u64, gpr: &mut VmExitGeneralPurposeRegister) {
//     let instruction = decode_one(rip).unwrap();
// }
