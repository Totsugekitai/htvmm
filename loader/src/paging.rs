use crate::{ALLOCATE_BYTES_FOR_VMM, MAX_ADDRESS};
use uefi::{
    prelude::BootServices,
    table::boot::{AllocateType, MemoryType},
};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{page_table::PageTableFlags, PageTable},
    PhysAddr,
};

pub fn create_page_table(vmm_entry_phys: PhysAddr, bs: &BootServices) -> (PhysAddr, Cr3Flags) {
    let (uefi_pml4, cr3_flags) = Cr3::read();
    let uefi_pml4_table = uefi_pml4.start_address().as_u64() as *const PageTable;
    unsafe {
        let vmm_pml4_table = construct_table(&*uefi_pml4_table, bs);
        modify_table(vmm_pml4_table, vmm_entry_phys, bs);

        (PhysAddr::new(vmm_pml4_table as *const _ as u64), cr3_flags)
    }
}

unsafe fn construct_table(
    uefi_pml4_table: &PageTable,
    bs: &BootServices,
) -> &'static mut PageTable {
    let vmm_pml4_table = bs
        .allocate_pages(
            AllocateType::MaxAddress(MAX_ADDRESS),
            MemoryType::UNUSABLE,
            1,
        )
        .unwrap();
    let vmm_pml4_table = core::mem::transmute::<u64, &mut PageTable>(vmm_pml4_table);
    vmm_pml4_table.zero();

    construct_table_inner(vmm_pml4_table, uefi_pml4_table, bs, 4);

    vmm_pml4_table
}

unsafe fn construct_table_inner(
    vmm_page_table: &mut PageTable,
    uefi_page_table: &PageTable,
    bs: &BootServices,
    level: usize,
) {
    for i in 0..512 {
        let uefi_table_entry = &uefi_page_table[i];
        if uefi_table_entry.is_unused() {
            continue;
        }

        if ((level == 2 || level == 3)
            && uefi_table_entry.flags().contains(PageTableFlags::HUGE_PAGE))
            || level == 1
        {
            vmm_page_table[i] = uefi_table_entry.clone();
            continue;
        }

        let sub_vmm_page_table = bs
            .allocate_pages(
                AllocateType::MaxAddress(MAX_ADDRESS),
                MemoryType::UNUSABLE,
                1,
            )
            .unwrap();
        let sub_vmm_page_table = core::mem::transmute::<u64, &mut PageTable>(sub_vmm_page_table);

        let sub_uefi_page_table = &*(uefi_table_entry.addr().as_u64() as *const PageTable);

        construct_table_inner(sub_vmm_page_table, sub_uefi_page_table, bs, level - 1);

        vmm_page_table[i].set_addr(
            PhysAddr::new(sub_vmm_page_table as *const PageTable as u64),
            uefi_table_entry.flags(),
        )
    }
}

unsafe fn modify_table(
    vmm_pml4_table: &mut PageTable,
    vmm_entry_phys: PhysAddr,
    bs: &BootServices,
) {
    // create 0x1_0000_0000 ~ linear address page table
    let pd_table = bs
        .allocate_pages(
            AllocateType::MaxAddress(MAX_ADDRESS),
            MemoryType::UNUSABLE,
            1,
        )
        .unwrap();
    let pd_table = core::mem::transmute::<u64, &mut PageTable>(pd_table);
    pd_table.zero();

    let vmm_entry_phys = vmm_entry_phys.as_u64() & 0xffff_ffff_ffe0_0000;
    for i in 0..(ALLOCATE_BYTES_FOR_VMM / 0x200000) {
        let pde = &mut pd_table[i];
        pde.set_addr(
            PhysAddr::new(vmm_entry_phys + 0x20_0000 * i as u64),
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::HUGE_PAGE
                | PageTableFlags::GLOBAL,
        );
    }

    let vmm_pdp_table =
        core::mem::transmute::<u64, &mut PageTable>((&vmm_pml4_table[0]).addr().as_u64());
    let vmm_pdpte4 = &mut vmm_pdp_table[4];
    let vmm_pdpte4_flags = vmm_pdpte4.flags();
    vmm_pdpte4.set_addr(
        PhysAddr::new(pd_table as *mut PageTable as u64),
        vmm_pdpte4_flags,
    );
}
