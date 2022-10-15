#![no_std]

mod arch;
mod cpu;
mod virt;

use crate::arch::intel::IntelCpu;
use common::BootArgs;
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
    ptr,
};
use cpu::Cpu;

global_asm!(include_str!("entry.s"), options(att_syntax));

#[no_mangle]
pub unsafe extern "C" fn vmm_main(boot_args: *const BootArgs) {
    let _boot_args = (&*boot_args).clone();
    clear_bss();
    let mut intel = IntelCpu::new();
    if intel.is_virtualization_supported() {
        for _ in 0..10000000 {
            asm!("nop");
        }
        let _ = intel.enable_virtualization();
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
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
