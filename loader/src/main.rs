#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use core::{arch::asm, mem::MaybeUninit};
use uefi::{
    prelude::*,
    proto::{
        console::text::{Input, Output},
        media::file::{File, FileAttribute, FileMode},
    },
    CStr16,
};

static mut STDIN: MaybeUninit<&mut Input> = MaybeUninit::uninit();
static mut STDOUT: MaybeUninit<&mut Output> = MaybeUninit::uninit();

#[entry]
fn efi_main(image_handle: Handle, mut systab: SystemTable<Boot>) -> Status {
    unsafe {
        STDIN.write(systab.stdin());
        STDOUT.write(systab.stdout());
    }

    let boot_services = systab.boot_services();
    let simple_fs = boot_services.get_image_file_system(image_handle);
    if simple_fs.is_err() {
        halt("[ERROR] SimpleFileSystem");
    }
    let simple_fs = simple_fs.unwrap();

    let volume = simple_fs.open_volume();
    if volume.is_err() {
        halt("[ERROR] OpenVolume");
    }
    let volume = volume.unwrap();

    let mut file_name_buf = [0; 0x20];
    let file_name = CStr16::from_str_with_buf("htvmm.elf", &mut file_name_buf).unwrap();
    let vmm_file_handle = volume.open(file_name, FileMode::Read, FileAttribute::empty());
    if vmm_file_handle.is_err() {
        halt("[ERROR] OpenFile");
    }
    let vmm_file_handle = vmm_file_handle.unwrap();

    Status::SUCCESS
}

fn halt(error_msg: &str) {
    let mut buf = [0; 0x1000];
    unsafe {
        STDOUT
            .assume_init_mut()
            .output_string(CStr16::from_str_with_buf(error_msg, &mut buf).unwrap())
            .unwrap();
    }
    loop {
        asm!("hlt");
    }
}
