#![no_std]
#![feature(default_alloc_error_handler)]

mod allocator;
mod arch;
mod cpu;
mod virt;

extern crate alloc;

use crate::arch::intel::IntelCpu;
use common::{BootArgs, VMM_AREA_HEAD_VADDR, VMM_AREA_SIZE, VMM_HEAP_HEAD_VADDR};
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};
use cpu::Cpu;

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
}

global_asm!(include_str!("entry.s"), options(att_syntax));

#[no_mangle]
pub unsafe extern "C" fn vmm_main(boot_args: *const BootArgs) {
    let _boot_args = (&*boot_args).clone();
    // allocator::init(
    //     VMM_HEAP_HEAD_VADDR,
    //     VMM_AREA_SIZE as usize - (VMM_HEAP_HEAD_VADDR - VMM_AREA_HEAD_VADDR),
    // );
    let mut intel = IntelCpu::new();
    if intel.is_virtualization_supported() {
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
