#![no_std]
#![no_main]
#![feature(abi_efiapi)]

mod paging;

#[macro_use]
extern crate alloc;

use crate::paging::create_page_table;
use common::{BootArgs, VMM_AREA_HEAD_VADDR, VMM_AREA_SIZE};
use core::{arch::asm, fmt::Write};
use goblin::elf;
use uefi::{
    data_types::Align,
    prelude::*,
    proto::{
        console::text::Output,
        media::file::{File, FileAttribute, FileInfo, FileMode},
    },
    table::boot::{AllocateType, MemoryType},
    CStr16,
};
use uefi_services::{self, println};
use x86_64::{structures::paging::PhysFrame, PhysAddr};

const VMM_FILE_NAME: &'static str = "htvmm.elf";
const PAGE_SIZE: usize = 0x1000;
pub const MAX_ADDRESS: usize = 0x4000_0000;
pub const VMM_ENTRY_VADDR: usize = 0x1_0000_1000;

#[entry]
fn efi_main(image_handle: Handle, mut systab: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut systab).unwrap();

    let boot_services = systab.boot_services();

    let memory_size = get_memory_size(boot_services);
    println!("Memory size: {}GB", memory_size / (1024 * 1024 * 1024));
    let uefi_write_char = Output::write_char as *const () as u64;
    let mut systab_clone = unsafe { systab.unsafe_clone() };
    let uefi_output = systab_clone.stdout() as *mut Output as u64;

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
    let vmm_page_count = VMM_AREA_SIZE as usize / PAGE_SIZE;
    let alloc_paddr = boot_services.allocate_pages(
        AllocateType::MaxAddress(MAX_ADDRESS as usize),
        MemoryType::UNUSABLE,
        vmm_page_count,
    );
    if alloc_paddr.is_err() {
        halt("[ERROR] allocate_pages");
    }
    let alloc_paddr = alloc_paddr.unwrap();
    // modify_uefi_page_table(PhysAddr::new(alloc_paddr), vmm_page_count);

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

    let (uefi_cr3, uefi_cr3_flags) = x86_64::registers::control::Cr3::read();
    let uefi_cr3_u64 = uefi_cr3.start_address().as_u64();
    let boot_args = BootArgs {
        uefi_cr3: PhysAddr::new(uefi_cr3_u64),
        uefi_cr3_flags,
        vmm_phys_offset: alloc_paddr as i64 - VMM_AREA_HEAD_VADDR as i64,
        memory_size,
        uefi_write_char,
        uefi_output,
    };

    let entry_point = alloc_paddr + vmm_entry_offset;

    println!("ENTER VMM: 0x{:x}", VMM_ENTRY_VADDR);

    let (vmm_pml4_table, cr3_flags) = create_page_table(PhysAddr::new(entry_point), boot_services);

    x86_64::instructions::interrupts::disable();
    unsafe {
        x86_64::registers::control::Cr3::write(
            PhysFrame::from_start_address(PhysAddr::new(vmm_pml4_table.as_u64())).unwrap(),
            cr3_flags,
        );
    }

    unsafe {
        asm!(
            "push %rax",
            "push %rbx",
            "push %rcx",
            "push %rdx",
            "push %rdi",
            "push %rsi",
            "push %r8",
            "push %r9",
            "push %r10",
            "push %r11",
            "push %r12",
            "push %r13",
            "push %r14",
            "push %r15",
            "mov {boot_args}, %rdi",
            "mov {vmm_entry}, %rax",
            "call *%rax",
            "pop %r15",
            "pop %r14",
            "pop %r13",
            "pop %r12",
            "pop %r11",
            "pop %r10",
            "pop %r9",
            "pop %r8",
            "pop %rsi",
            "pop %rdi",
            "pop %rdx",
            "pop %rcx",
            "pop %rbx",
            "pop %rax",
            boot_args = in(reg) &boot_args as *const BootArgs,
            vmm_entry = in(reg) VMM_ENTRY_VADDR,
            options(att_syntax)
        );
        x86_64::registers::control::Cr3::write(uefi_cr3, uefi_cr3_flags);
    }
    x86_64::instructions::hlt();
    x86_64::instructions::interrupts::enable();

    halt("VMM boot OK!");

    // Status::SUCCESS
}

fn halt(error_msg: &str) -> ! {
    println!("{error_msg}");
    x86_64::instructions::interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

fn get_memory_size(bs: &BootServices) -> u64 {
    let mut size = 0;
    loop {
        size += 0x100;
        let pool = bs.allocate_pool(MemoryType::UNUSABLE, size).unwrap();
        let buf = unsafe { core::slice::from_raw_parts_mut(pool, size) };
        let memmap = bs.memory_map(buf);
        if let Ok((_mapkey, memdesc_iter)) = memmap {
            let mut memory_size = 0;
            for memdesc in memdesc_iter {
                let phys_end = memdesc.phys_start + (0x1000 * memdesc.page_count);
                if memory_size < phys_end {
                    memory_size = phys_end;
                }
            }
            bs.free_pool(pool).unwrap();
            return memory_size;
        } else {
            bs.free_pool(pool).unwrap();
            continue;
        }
    }
}
