#![no_std]
#![feature(default_alloc_error_handler)]

mod allocator;
mod arch;
mod cpu;

extern crate alloc;

use crate::arch::intel::IntelCpu;
use alloc::vec;
use common::{BootArgs, VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE};
use core::{arch::global_asm, panic::PanicInfo};
use cpu::Cpu;

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
}

global_asm!(include_str!("entry.s"), options(att_syntax));

#[no_mangle]
pub unsafe extern "C" fn vmm_main(boot_args: *const BootArgs) {
    let _boot_args = (&*boot_args).clone();
    allocator::init(VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE as usize);
    let mut intel = IntelCpu::new();
    let v = vec![1; 0x1000];
    for _ in v {
        x86_64::instructions::nop();
    }
    if intel.is_virtualization_supported() {
        let _ = intel.enable_virtualization();
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
