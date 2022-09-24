#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub fn vmm_main() {}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
