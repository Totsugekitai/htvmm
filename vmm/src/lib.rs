#![no_std]
#![feature(default_alloc_error_handler)]

mod allocator;
mod arch;
mod cpu;
mod emu;
mod ioapic;
mod serial;

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

/// # Safety
/// This function is unsafe.
#[no_mangle]
pub unsafe extern "sysv64" fn vmm_main(boot_args: *const BootArgs) {
    clear_bss();
    BOOT_ARGS.store(*boot_args);
    UEFI_WRITE_CHAR.store(BOOT_ARGS.load().uefi_write_char);
    allocator::init(VMM_HEAP_HEAD_VADDR, VMM_HEAP_SIZE as usize);
    serial::init(serial::COM);

    serial_println!("VMM init complete");

    let mut intel = IntelCpu::new();

    if let Err(e) = intel.enable_virtualization() {
        panic!("failed to enable virtualization: {e:?}");
    }
    intel.init_as_bsp();
    intel.run_vm();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("{info:?}");
    loop {
        x86_64::instructions::hlt();
    }
}

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    // fn call_uefi_write_char(fp: u64, output: u64, c: u32);
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

// fn _print_uefi(args: core::fmt::Arguments) {
//     let uefi_write_char = UEFI_WRITE_CHAR.load();
//     let output = BOOT_ARGS.load().uefi_output;
//     let s = format(args);
//     let s = s.as_str();
//     for c in s.chars() {
//         unsafe {
//             call_uefi_write_char(uefi_write_char, output, c as u32);
//         }
//     }
// }

// #[macro_export]
// macro_rules! uefi_print {
//     ($($arg:tt)*) => ($crate::_print_uefi(core::format_args!($($arg)*)));
// }

// #[macro_export]
// macro_rules! uefi_println {
//     () => ($crate::uefi_print!("\r\n"));
//     ($($arg:tt)*) => ($crate::_print_uefi(core::format_args!("{}{}", core::format_args!($($arg)*), "\r\n")));
// }

fn _print_serial_asm(args: core::fmt::Arguments) {
    for c in format(args).as_str().chars() {
        unsafe {
            use core::arch::asm;
            asm!("out dx, al", in("dx") 0x3e8, in("al") c as u8, options(nomem, nostack));
        }
    }
}

fn _print_serial(args: core::fmt::Arguments) {
    let s = format(args);
    let s = s.as_str();
    for c in s.chars() {
        unsafe {
            serial::write(serial::COM, c as u8);
        }
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::_print_serial(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => ($crate::_print_serial(core::format_args!("{}{}", core::format_args!($($arg)*), "\r\n")));
}
