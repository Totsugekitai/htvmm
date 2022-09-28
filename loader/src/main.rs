#![no_std]
#![no_main]
#![feature(abi_efiapi)]

#[macro_use]
extern crate alloc;

use common::{BootArgs, PhysAddr};
use core::arch::asm;
use goblin::elf;
use uefi::{
    data_types::Align,
    prelude::*,
    proto::media::file::{File, FileAttribute, FileInfo, FileMode},
    table::boot::{AllocateType, MemoryType},
    CStr16,
};
use uefi_services::{self, println};

const VMM_FILE_NAME: &'static str = "htvmm.elf";
const PAGE_SIZE: usize = 0x1000;
const MAX_ADDRESS: usize = 0x40000000;

#[entry]
fn efi_main(image_handle: Handle, mut systab: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut systab).unwrap();

    let boot_services = systab.boot_services();
    let simple_fs = boot_services.get_image_file_system(image_handle);
    if simple_fs.is_err() {
        halt("[ERROR] SimpleFileSystem");
    }
    let mut simple_fs = simple_fs.unwrap();

    let volume = simple_fs.open_volume();
    if volume.is_err() {
        halt("[ERROR] OpenVolume");
    }
    let mut volume = volume.unwrap();

    let mut file_name_buf = [0; 0x20];
    let file_name = CStr16::from_str_with_buf(VMM_FILE_NAME, &mut file_name_buf).unwrap();
    let vmm_file_handle = volume.open(file_name, FileMode::Read, FileAttribute::empty());
    if vmm_file_handle.is_err() {
        halt("[ERROR] OpenFile");
    }
    let mut vmm_file_handle = vmm_file_handle.unwrap();

    let mut info_buf = vec![0; 0x1000];
    let info_buf = FileInfo::align_buf(&mut info_buf);
    if info_buf.is_none() {
        halt("[ERROR] alignment file info");
    }
    let info_buf = info_buf.unwrap();
    let file_info = vmm_file_handle.get_info::<FileInfo>(info_buf);
    if file_info.is_err() {
        halt("[ERROR] get FileInfo");
    }
    let file_info = file_info.unwrap();

    let file_size = file_info.file_size();
    let alloc_paddr = boot_services.allocate_pages(
        AllocateType::MaxAddress(MAX_ADDRESS as usize),
        MemoryType::UNUSABLE,
        (file_size as usize + PAGE_SIZE - 1) / PAGE_SIZE,
    );
    if alloc_paddr.is_err() {
        halt("[ERROR] allocate_pages");
    }
    let alloc_paddr = alloc_paddr.unwrap();

    let vmm_regular_file = vmm_file_handle.into_regular_file();
    if vmm_regular_file.is_none() {
        halt("[ERROR] into_regular_file");
    }
    let mut vmm_regular_file = vmm_regular_file.unwrap();
    let region =
        unsafe { core::slice::from_raw_parts_mut(alloc_paddr as *mut u8, file_size as usize) };
    let read_res = vmm_regular_file.read(region);
    if read_res.is_err() {
        halt("[ERROR] read");
    }

    let vmm_elf = elf::Elf::parse(&region);
    if vmm_elf.is_err() {
        halt("[ERROR] parse ELF");
    }
    let vmm_elf = vmm_elf.unwrap();
    let vmm_entry_offset = vmm_elf.program_headers[0].p_offset; // FIXME!!!

    let map_paddr = PhysAddr::new(alloc_paddr);

    let boot_args = BootArgs {
        file_size,
        map_paddr,
    };

    let vmm_entry: extern "sysv64" fn(*const BootArgs) =
        unsafe { core::mem::transmute(alloc_paddr + vmm_entry_offset) };

    println!("ENTER VMM");
    vmm_entry(&boot_args as *const BootArgs); // enter VMM!!!
    println!("[OK] vmm_entry");

    Status::SUCCESS
}

fn halt(error_msg: &str) -> ! {
    println!("{error_msg}");
    loop {
        unsafe { asm!("hlt") };
    }
}
