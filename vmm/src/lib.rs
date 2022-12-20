#![no_std]
#![feature(default_alloc_error_handler)]

mod allocator;
mod arch;
mod cpu;

extern crate alloc;

use crate::arch::intel::IntelCpu;
use alloc::fmt::format;
use common::{BootArgs, VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE};
use core::{arch::global_asm, panic::PanicInfo, ptr};
use cpu::Cpu;
use crossbeam::atomic::AtomicCell;

global_asm!(include_str!("entry.s"), options(att_syntax));

pub static BOOT_ARGS: AtomicCell<BootArgs> = AtomicCell::new(BootArgs::new());
pub static UEFI_WRITE_CHAR: AtomicCell<u64> = AtomicCell::new(0);

#[no_mangle]
pub unsafe extern "sysv64" fn vmm_main(boot_args: *const BootArgs) {
    clear_bss();
    BOOT_ARGS.store(*boot_args);
    UEFI_WRITE_CHAR.store(BOOT_ARGS.as_ptr().as_ref().unwrap().uefi_write_char);
    allocator::init(VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE as usize);

    println!("VMM init complete");

    let mut intel = IntelCpu::new();

    if let Err(_e) = intel.enable_virtualization() {
        panic!();
    }
    intel.init_as_bsp();
    intel.run_vm();
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
    fn call_uefi_write_char(fp: u64, output: u64, c: u32);
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

fn _print(args: core::fmt::Arguments) {
    let uefi_write_char = UEFI_WRITE_CHAR.load();
    let output = BOOT_ARGS.load().uefi_output;
    let s = format(args);
    let s = s.as_str();
    for c in s.chars() {
        unsafe {
            call_uefi_write_char(uefi_write_char, output, c as u32);
        }
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::_print(core::format_args!("{}{}", core::format_args!($($arg)*), "\n")));
}
