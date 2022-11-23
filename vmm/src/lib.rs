#![no_std]
#![feature(default_alloc_error_handler)]

mod allocator;
mod arch;
mod cpu;

extern crate alloc;

use crate::arch::intel::IntelCpu;
use common::{BootArgs, VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE};
use core::{arch::global_asm, panic::PanicInfo, ptr};
use cpu::Cpu;
use crossbeam::atomic::AtomicCell;

global_asm!(include_str!("entry.s"), options(att_syntax));

pub static BOOT_ARGS: AtomicCell<BootArgs> = AtomicCell::new(BootArgs::new());

#[no_mangle]
pub unsafe extern "sysv64" fn vmm_main(boot_args: *const BootArgs) {
    clear_bss();
    // let boot_args = boot_args.as_ref().unwrap().clone();
    BOOT_ARGS.store(*boot_args);
    allocator::init(VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE as usize);

    let mut intel = IntelCpu::new();

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

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
}

unsafe fn clear_bss() {
    let bss = &__bss as *const u8 as u64;
    let bss_end = &__bss_end as *const u8 as u64;
    let start = bss;
    let count = (bss_end - bss) / 8;
    for i in 0..count {
        let dst = (start + i * 8) as *mut u64;
        ptr::write_volatile(dst, 0);
    }
}
