use core::arch::asm;
use x86_64::{
    instructions::tables::sgdt,
    registers::segmentation::Segment,
    structures::gdt::SegmentSelector,
    structures::paging::{PageTable, PageTableFlags},
    PhysAddr,
};

pub trait Cpu {
    fn is_virtualization_supported(&self) -> bool;
    fn enable_virtualization(&mut self) -> Result<(), CpuError>;
    fn disable_virtualization(&mut self) -> Result<(), CpuError>;
    fn init_as_bsp(&mut self);
    fn run_vm(&mut self);
}

#[derive(Debug)]
pub enum CpuError {
    NotSupported,
    VmxRelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ldtr;

impl Segment for Ldtr {
    fn get_reg() -> SegmentSelector {
        let mut ldtr: u16 = 0;
        unsafe {
            asm!("sldt [{}]", in(reg) &mut ldtr, options(nostack, preserves_flags));
        }
        SegmentSelector(ldtr)
    }

    unsafe fn set_reg(sel: SegmentSelector) {
        asm!("lldt [{}]", in(reg) &sel.0, options(readonly, nostack, preserves_flags));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tr;

impl Segment for Tr {
    fn get_reg() -> SegmentSelector {
        let tr: u16;
        unsafe {
            asm!("str {0:x}", out(reg) tr, options(nostack, nomem, preserves_flags));
        }
        SegmentSelector(tr)
    }

    unsafe fn set_reg(sel: SegmentSelector) {
        asm!("ltr [{}]", in(reg) &sel.0, options(readonly, nostack, preserves_flags));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentDescriptor;

impl SegmentDescriptor {
    pub fn base(sel: &SegmentSelector) -> u64 {
        let i = sel.index() as usize;
        let gdt = sgdt();
        let gdt_limit = gdt.limit;
        let gdt = unsafe {
            core::slice::from_raw_parts(gdt.base.as_ptr() as *const u64, gdt_limit as usize)
        };
        let segment_descriptor = gdt[i];
        let segment_descriptor_high = gdt[i + 1];

        ((segment_descriptor_high & 0xffff_ffff) << 32)
            | ((segment_descriptor & (0xff << 56)) >> 32)
            | ((segment_descriptor & (0xff << 32)) >> 16)
            | ((segment_descriptor & (0xffff << 16)) >> 16)
    }

    // pub fn limit(sel: &SegmentSelector) -> u32 {
    //     let mut limit: u32 = 0;
    //     unsafe {
    //         asm!("lsl {}, [{}]", in(reg) &mut limit, in(reg) &sel.0, options(nostack, preserves_flags));
    //     }
    //     limit
    // }
}

pub fn guest_virt_to_guest_phys(guest_virt: u64, guest_cr3: u64) -> PhysAddr {
    let pml4_index = ((guest_virt >> 39) & 0b1_1111_1111) as usize;
    let pdp_index = ((guest_virt >> 30) & 0b1_1111_1111) as usize;
    let pd_index = ((guest_virt >> 21) & 0b1_1111_1111) as usize;
    let pt_index = ((guest_virt >> 12) & 0b1_1111_1111) as usize;
    // serial_println!("pml4: {pml4_index}, pdp: {pdp_index}, pd: {pd_index}, pt: {pt_index}");

    let guest_pml4_phys = PhysAddr::new(guest_cr3 & !0b1111_1111_1111);
    let guest_pml4 = unsafe {
        (guest_pml4_phys.as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    // use crate::serial_println;

    let guest_pml4_entry = &guest_pml4[pml4_index];
    // serial_println!("{:?}", guest_pml4_entry);
    if !guest_pml4_entry.flags().contains(PageTableFlags::PRESENT) {
        return PhysAddr::new(0);
    }

    let guest_pdp = unsafe {
        (guest_pml4_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pdp_entry = &guest_pdp[pdp_index];
    // serial_println!("{:?}", guest_pdp_entry);
    if !guest_pdp_entry.flags().contains(PageTableFlags::PRESENT) {
        return PhysAddr::new(0);
    }
    if guest_pdp_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
        let pdp_offset = guest_virt & 0b1_1111_1111_1111_1111_1111_1111_1111;
        return guest_pdp_entry.addr() + pdp_offset;
    }

    let guest_pd = unsafe {
        (guest_pdp_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pd_entry = &guest_pd[pd_index];
    // serial_println!("{:?}", guest_pd_entry);
    if !guest_pd_entry.flags().contains(PageTableFlags::PRESENT) {
        return PhysAddr::new(0);
    }
    if guest_pd_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
        let pd_offset = guest_virt & 0b1_1111_1111_1111_1111_1111;
        return guest_pd_entry.addr() + pd_offset;
    }

    let guest_pt = unsafe {
        (guest_pd_entry.addr().as_u64() as *mut PageTable)
            .as_mut()
            .unwrap()
    };

    let guest_pt_entry = &guest_pt[pt_index];
    let pt_offset = guest_virt & 0b1111_1111_1111;
    guest_pt_entry.addr() + pt_offset
}
