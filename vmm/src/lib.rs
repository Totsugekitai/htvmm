#![no_std]
#![no_main]

use common::{BootArgs, PhysAddr};
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
    ptr,
};

global_asm!(include_str!("entry.s"), options(att_syntax));

#[no_mangle]
pub extern "C" fn vmm_main(boot_args: *const BootArgs) {
    let boot_args = unsafe { &*boot_args };
    unsafe {
        clear_bss(&boot_args.map_paddr);
        // loop {
        //     asm!("hlt");
        // }
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

unsafe fn clear_bss(map_paddr: &PhysAddr) {
    let bss = &__bss as *const u8 as u64;
    let bss_end = &__bss_end as *const u8 as u64;
    let map_paddr = map_paddr.to_u64();
    let start = map_paddr + bss;
    let count = (bss_end - bss) / 8;
    for i in 0..count {
        let dst = (start + i * 8) as *mut u64;
        ptr::write_volatile(dst, 0);
    }
}
