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
use crossbeam::atomic::AtomicCell;

global_asm!(include_str!("entry.s"), options(att_syntax));

pub static BOOT_ARGS: AtomicCell<BootArgs> = AtomicCell::new(BootArgs::new());

#[no_mangle]
pub unsafe extern "C" fn vmm_main(boot_args: *const BootArgs) {
    BOOT_ARGS.store(boot_args.as_ref().unwrap().clone());
    allocator::init(VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE as usize);

    let mut intel = IntelCpu::new();
    let v = vec![1; 0x1000];
    for _ in v {
        x86_64::instructions::nop();
    }

    if let Err(_e) = intel.enable_virtualization() {
        panic!();
    }
    intel.init_as_bsp();
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
