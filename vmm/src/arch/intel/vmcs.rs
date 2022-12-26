use crate::{
    arch::intel::{
        ept::EptPointer,
        vmx::{vmclear, vmptrld, vmread, vmwrite},
    },
    cpu::{Ldtr, SegmentDescriptor, Tr},
    BOOT_ARGS,
};
use alloc::alloc::alloc;
use common::constants;
use core::{alloc::Layout, ptr, slice};
use x86_64::{
    instructions::tables::{sgdt, sidt},
    registers::{
        control::{Cr0, Cr3, Cr4, Cr4Flags},
        debug::Dr7,
        model_specific::Msr,
        rflags,
        segmentation::{Segment, Segment64, CS, DS, ES, FS, GS, SS},
    },
    structures::DescriptorTablePointer,
    PhysAddr, VirtAddr,
};

extern "C" {
    static entry_ret: u8;
    static uefi_cs: u16;
    static uefi_ds: u16;
    static uefi_es: u16;
    static uefi_fs: u16;
    static uefi_gs: u16;
    static uefi_ss: u16;
    static uefi_gdtr: u8;
    static uefi_idtr: u8;
    static uefi_ldtr: u64;
    static uefi_tr: u64;
    static uefi_rsp: u64;
    static uefi_cr3: u64;
}

#[derive(Debug)]
pub struct VmcsRegion(*mut u8);

unsafe impl Send for VmcsRegion {}

impl VmcsRegion {
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align(4096, 4096).unwrap();
        let region = alloc(layout);
        let region_slice = slice::from_raw_parts_mut(region, 4096);
        region_slice.fill(0);

        let ia32_vmx_basic = Msr::new(constants::MSR_IA32_VMX_BASIC).read();
        let vmcs_rev_id = (ia32_vmx_basic & 0x7fff_ffff) as u32;

        ptr::write_volatile(region as *mut u32, vmcs_rev_id);

        Self(region)
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0
    }

    pub fn paddr(&self) -> PhysAddr {
        let virt = i64::try_from(self.as_mut_ptr() as u64).unwrap();
        let phys = unsafe {
            u64::try_from(virt + BOOT_ARGS.as_ptr().as_ref().unwrap().vmm_phys_offset).unwrap()
        };
        PhysAddr::new(phys)
    }

    pub fn clear(&mut self) {
        unsafe {
            vmclear(self);
        }
    }

    pub fn load(&self) {
        unsafe {
            vmptrld(self);
        }
    }

    pub fn read(&self, field: VmcsField) -> u64 {
        unsafe { vmread(field) }
    }

    pub fn write(&mut self, field: VmcsField, val: u64) {
        unsafe {
            vmwrite(field, val);
        }
    }

    pub fn setup(&mut self, eptp: EptPointer, vmexit_host_rip: u64) {
        self.setup_guest_state_area();
        self.setup_host_state_area(vmexit_host_rip);
        self.setup_vm_control_fields(eptp);
    }

    fn setup_guest_state_area(&mut self) {
        // 16 bit guest state fields
        let cs = CS::get_reg();
        let ds = DS::get_reg();
        let es = ES::get_reg();
        let fs = FS::get_reg();
        let gs = GS::get_reg();
        let ss = SS::get_reg();
        let ldtr = Ldtr::get_reg();
        let tr = Tr::get_reg();
        let cs = unsafe { uefi_cs };
        let ds = unsafe { uefi_ds };
        let es = unsafe { uefi_es };
        let fs = unsafe { uefi_fs };
        let gs = unsafe { uefi_gs };
        let ss = unsafe { uefi_ss };
        let ldtr = unsafe { uefi_ldtr };
        let tr = unsafe { uefi_tr };
        self.write(VmcsField::GuestCsSelector, cs as u64);
        self.write(VmcsField::GuestDsSelector, ds as u64);
        self.write(VmcsField::GuestEsSelector, es as u64);
        self.write(VmcsField::GuestFsSelector, fs as u64);
        self.write(VmcsField::GuestGsSelector, gs as u64);
        self.write(VmcsField::GuestSsSelector, ss as u64);
        self.write(VmcsField::GuestLdtrSelector, ldtr);
        self.write(VmcsField::GuestTrSelector, tr);

        // 32 bit guest state fields
        let gdtr = unsafe {
            let limit = *(&uefi_gdtr as *const u8 as *const u16);
            let base = *((&uefi_gdtr as *const u8).offset(2) as *const u64);
            let gdtr = DescriptorTablePointer {
                limit,
                base: VirtAddr::new(base),
            };
            gdtr
        };
        let idtr = unsafe {
            let limit = *(&uefi_idtr as *const u8 as *const u16);
            let base = *((&uefi_idtr as *const u8).offset(2) as *const u64);
            let idtr = DescriptorTablePointer {
                limit,
                base: VirtAddr::new(base),
            };
            idtr
        };
        let gdtr_limit = gdtr.limit;
        let idtr_limit = idtr.limit;
        let sysenter_cs = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_CS).read() };
        let efer = unsafe { Msr::new(constants::MSR_EFER).read() };
        self.write(VmcsField::GuestCsLimit, 0xffffffff);
        self.write(VmcsField::GuestDsLimit, 0xffffffff);
        self.write(VmcsField::GuestEsLimit, 0xffffffff);
        self.write(VmcsField::GuestFsLimit, 0xffffffff);
        self.write(VmcsField::GuestGsLimit, 0xffffffff);
        self.write(VmcsField::GuestSsLimit, 0xffffffff);
        self.write(VmcsField::GuestLdtrLimit, 0xffff);
        self.write(VmcsField::GuestTrLimit, 0xffff);
        self.write(VmcsField::GuestGdtrLimit, gdtr_limit as u64);
        self.write(VmcsField::GuestIdtrLimit, idtr_limit as u64);
        self.write(VmcsField::GuestCsAccessRights, 0xa09b);
        self.write(VmcsField::GuestDsAccessRights, 0xc093);
        self.write(VmcsField::GuestEsAccessRights, 0xc093);
        self.write(VmcsField::GuestFsAccessRights, 0xc093);
        self.write(VmcsField::GuestGsAccessRights, 0xc093);
        self.write(VmcsField::GuestSsAccessRights, 0xc093);
        self.write(VmcsField::GuestLdtrAccessRights, 0x0082);
        self.write(VmcsField::GuestTrAccessRights, 0x008b);
        self.write(VmcsField::GuestInterruptibilityState, 0);
        self.write(VmcsField::GuestActivityState, 0);
        self.write(VmcsField::GuestPendingDbgExceptions, 0);
        self.write(VmcsField::GuestIa32SysenterCs, sysenter_cs);

        // 64 bit guest state fields
        self.write(VmcsField::VmcsLinkPointer, 0xffffffff_ffffffffu64);
        self.write(VmcsField::GuestIa32Debugctl, 0u64);
        self.write(VmcsField::GuestIa32Efer, efer);

        // natural width guest state fields
        let cr0 = Cr0::read_raw();
        let cr3_tuple = Cr3::read_raw();
        let cr3 = cr3_tuple.0.start_address().as_u64() | (cr3_tuple.1 as u64);
        let cr4 = Cr4::read_raw();
        // let ldtr_base = SegmentDescriptor::base(&ldtr);
        // let tr_base = SegmentDescriptor::base(&tr);
        let gdtr_base = gdtr.base.as_u64();
        let idtr_base = idtr.base.as_u64();
        let dr7 = Dr7::read_raw();
        // VM(bit 17) must be 0, IF(bit 9) must be 1, IF is 0
        let rflags =
            (rflags::read_raw() & !(1 << 17 | rflags::RFlags::INTERRUPT_FLAG.bits())) | (1 << 9);
        let sysenter_esp = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_ESP).read() };
        let sysenter_eip = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_EIP).read() };
        let rsp = unsafe { uefi_rsp };
        let rip = unsafe { &entry_ret as *const u8 as u64 };
        // let rip = unsafe {
        //     &entry_ret as *const u8 as u64 as i64
        //         + BOOT_ARGS.as_ptr().as_ref().unwrap().vmm_phys_offset
        // } as u64;
        // let cr3 = unsafe { BOOT_ARGS.as_ptr().as_ref().unwrap().uefi_cr3.as_u64() };
        self.write(VmcsField::GuestCr0, cr0);
        self.write(VmcsField::GuestCr3, cr3);
        self.write(VmcsField::GuestCr4, cr4);
        self.write(VmcsField::GuestCsBase, 0);
        self.write(VmcsField::GuestDsBase, 0);
        self.write(VmcsField::GuestEsBase, 0);
        self.write(VmcsField::GuestFsBase, 0);
        self.write(VmcsField::GuestGsBase, 0);
        self.write(VmcsField::GuestSsBase, 0);
        self.write(VmcsField::GuestLdtrBase, 0);
        self.write(VmcsField::GuestTrBase, 0);
        self.write(VmcsField::GuestGdtrBase, gdtr_base);
        self.write(VmcsField::GuestIdtrBase, idtr_base);
        self.write(VmcsField::GuestDr7, dr7);
        self.write(VmcsField::GuestRsp, rsp);
        self.write(VmcsField::GuestRip, rip);
        self.write(VmcsField::GuestRflags, rflags);
        self.write(VmcsField::GuestSysenterEsp, sysenter_esp);
        self.write(VmcsField::GuestSysenterEip, sysenter_eip);
        // unsafe {
        //     use core::arch::asm;
        //     asm!("mov r15, {}; hlt", in(reg) idtr_limit as u64, options(readonly, nostack, preserves_flags));
        // };
    }

    fn setup_host_state_area(&mut self, vmexit_host_rip: u64) {
        // 16 bit host state fields
        let cs = CS::get_reg();
        let ds = DS::get_reg();
        let es = ES::get_reg();
        let fs = FS::get_reg();
        let gs = GS::get_reg();
        let ss = SS::get_reg();
        let tr = Tr::get_reg();
        self.write(VmcsField::HostCsSelector, cs.0 as u64);
        self.write(VmcsField::HostDsSelector, ds.0 as u64);
        self.write(VmcsField::HostEsSelector, es.0 as u64);
        self.write(VmcsField::HostFsSelector, fs.0 as u64);
        self.write(VmcsField::HostGsSelector, gs.0 as u64);
        self.write(VmcsField::HostSsSelector, ss.0 as u64);
        self.write(VmcsField::HostTrSelector, tr.0 as u64);

        // 32 bit host state fields
        let sysenter_cs = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_CS).read() };
        self.write(VmcsField::HostIa32SysenterCs, sysenter_cs);

        // native width host state fields
        let cr3_tuple = Cr3::read_raw();
        let cr3 = cr3_tuple.0.start_address().as_u64() | (cr3_tuple.1 as u64);
        // If the “host address-space size” VM-exit control is 1, Bit 5 of the CR4 field (corresponding to CR4.PAE) is 1.
        let cr4 = Cr4::read() | Cr4Flags::FSGSBASE | Cr4Flags::PHYSICAL_ADDRESS_EXTENSION;
        unsafe {
            Cr4::write(cr4);
        }
        let cr4 = Cr4::read_raw();
        // If bit 23 in the CR4 field (corresponding to CET) is 1, bit 16 in the CR0 field (WP) must also be 1.
        if (cr4 >> 23) & 1 == 1 {
            let cr0 = Cr0::read_raw();
            unsafe {
                Cr0::write_raw(cr0 | (1 << 16));
            }
        }
        let cr0 = Cr0::read_raw();
        let gdtr = sgdt();
        let idtr = sidt();
        let fs_base = FS::read_base().as_u64();
        let gs_base = GS::read_base().as_u64();
        let tr_base = SegmentDescriptor::base(&tr);
        let gdtr_base = gdtr.base.as_u64();
        let idtr_base = idtr.base.as_u64();
        let sysenter_esp = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_ESP).read() };
        let sysenter_eip = unsafe { Msr::new(constants::MSR_IA32_SYSENTER_EIP).read() };
        let efer = unsafe { Msr::new(constants::MSR_EFER).read() };
        let pat = unsafe { Msr::new(constants::MSR_IA32_CR_PAT).read() };
        let layout = Layout::from_size_align(4096, 16).unwrap();
        let stack_bottom = unsafe { alloc(layout) };
        let stack_top = stack_bottom as u64 + 4096 - 16;
        self.write(VmcsField::HostCr0, cr0);
        self.write(VmcsField::HostCr3, cr3);
        self.write(VmcsField::HostCr4, cr4);
        self.write(VmcsField::HostFsBase, fs_base);
        self.write(VmcsField::HostGsBase, gs_base);
        self.write(VmcsField::HostTrBase, tr_base);
        self.write(VmcsField::HostGdtrBase, gdtr_base);
        self.write(VmcsField::HostIdtrBase, idtr_base);
        self.write(VmcsField::HostIa32SysenterEsp, sysenter_esp);
        self.write(VmcsField::HostIa32SysenterEip, sysenter_eip);
        self.write(VmcsField::HostRsp, stack_top);
        self.write(VmcsField::HostRip, vmexit_host_rip);
        self.write(VmcsField::HostIa32Efer, efer);
        self.write(VmcsField::HostIa32Pat, pat);
        // unsafe {
        //     use core::arch::asm;
        //     asm!("mov r15, {}; hlt", in(reg) host_rip, options(readonly, nostack, preserves_flags));
        // };
    }

    fn setup_vm_control_fields(&mut self, eptp: EptPointer) {
        // 32 bit control fields
        let pin_based_ctls = unsafe { Msr::new(constants::MSR_IA32_VMX_PINBASED_CTLS).read() };
        let pin_based_ctls_or = (pin_based_ctls & 0xffff_ffff) as u32;
        let pin_based_ctls_and = ((pin_based_ctls >> 32) & 0xffff_ffff) as u32;
        let pin_based_ctls = ((0 | pin_based_ctls_or) & pin_based_ctls_and) as u64;
        let proc_based_ctls = unsafe { Msr::new(constants::MSR_IA32_VMX_PROCBASED_CTLS).read() };
        let proc_based_ctls_or = (proc_based_ctls & 0xffff_ffff) as u32;
        let proc_based_ctls_and = ((proc_based_ctls >> 32) & 0xffff_ffff) as u32;
        let proc_based_ctls = ((0 | proc_based_ctls_or) & proc_based_ctls_and) as u64;
        let proc_based_ctls2 = unsafe { Msr::new(constants::MSR_IA32_VMX_PROCBASED_CTLS2).read() };
        let proc_based_ctls2_or = (proc_based_ctls2 & 0xffff_ffff) as u32;
        let proc_based_ctls2_and = ((proc_based_ctls2 >> 32) & 0xffff_ffff) as u32;
        let proc_based_ctls2 = ((0 | proc_based_ctls2_or) & proc_based_ctls2_and) as u64;
        let exit_ctls = unsafe { Msr::new(constants::MSR_IA32_VMX_EXIT_CTLS).read() };
        let exit_ctls_or = (exit_ctls & 0xffff_ffff) as u32;
        let exit_ctls_and = ((exit_ctls >> 32) & 0xffff_ffff) as u32;
        let exit_ctls = ((0 | exit_ctls_or) & exit_ctls_and) as u64;
        let entry_ctls = unsafe { Msr::new(constants::MSR_IA32_VMX_ENTRY_CTLS).read() };
        let entry_ctls_or = (entry_ctls & 0xffff_ffff) as u32;
        let entry_ctls_and = ((entry_ctls >> 32) & 0xffff_ffff) as u32;
        let entry_ctls = ((0 | entry_ctls_or) & entry_ctls_and) as u64;
        self.write(VmcsField::PinBasedVmExecControls, pin_based_ctls);
        self.write(
            VmcsField::ProcBasedVmExecControls,
            proc_based_ctls
                | VMCS_PROC_BASED_VMEXEC_CTLS_HLTEXIT
                | VMCS_PROC_BASED_VMEXEC_CTLS_ACTIVE_SECOND_CTLS,
        );
        self.write(
            VmcsField::ProcBasedVmExecControls2,
            proc_based_ctls2 | VMCS_PROC_BASED_VMEXEC_CTLS2_ENABLE_EPT,
        );
        self.write(VmcsField::ExceptionBitmap, 0);
        self.write(VmcsField::PageFaultErrorCodeMask, 0);
        self.write(VmcsField::PageFaultErrorCodeMatch, 0);
        self.write(VmcsField::Cr3TargetCount, 0);
        self.write(
            VmcsField::VmExitControls,
            exit_ctls | VMCS_VMEXIT_CTLS_HOST_ADDR_SPACE_SIZE,
        );
        self.write(VmcsField::VmExitMsrStoreCount, 0);
        self.write(VmcsField::VmExitMsrLoadCount, 0);
        self.write(
            VmcsField::VmEntryControls,
            entry_ctls | VMCS_VMENTRY_CTLS_IA32E_MODE_GUEST,
        );
        self.write(VmcsField::VmEntryMsrLoadCount, 0);
        self.write(VmcsField::VmEntryIntrInfoField, 0);
        self.write(VmcsField::VmEntryExceptionErrorCode, 0);
        self.write(VmcsField::VmEntryInstructionLen, 0);
        self.write(VmcsField::TprThreshold, 0);

        // 64 bit control fields
        self.write(VmcsField::VmExitMsrLoadAddr, 0);
        self.write(VmcsField::VmExitMsrStoreAddr, 0);
        self.write(VmcsField::VmEntryMsrLoadAddr, 0);
        self.write(VmcsField::TscOffset, 0);
        self.write(VmcsField::EptPointer, eptp.as_u64());

        // natural width control fields
        // let cr0 = Cr0::read_raw();
        // let cr4 = Cr4::read_raw();
        self.write(VmcsField::Cr0GuestHostMask, 0);
        self.write(VmcsField::Cr4GuestHostMask, 0);
        self.write(VmcsField::Cr0ReadShadow, 0);
        self.write(VmcsField::Cr4ReadShadow, 0);
        // self.write(VmcsField::Cr3TargetValue0, 0); // hung
        // self.write(VmcsField::Cr3TargetValue1, 0); // hung
        // self.write(VmcsField::Cr3TargetValue2, 0); // hung
        // self.write(VmcsField::Cr3TargetValue3, 0); // hung
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VmcsField {
    GuestEsSelector = 0x00000800,
    GuestCsSelector = 0x00000802,
    GuestSsSelector = 0x00000804,
    GuestDsSelector = 0x00000806,
    GuestFsSelector = 0x00000808,
    GuestGsSelector = 0x0000080a,
    GuestLdtrSelector = 0x0000080c,
    GuestTrSelector = 0x0000080e,
    HostEsSelector = 0x00000c00,
    HostCsSelector = 0x00000c02,
    HostSsSelector = 0x00000c04,
    HostDsSelector = 0x00000c06,
    HostFsSelector = 0x00000c08,
    HostGsSelector = 0x00000c0a,
    HostTrSelector = 0x00000c0c,
    IoBitmapA = 0x00002000,
    IoBitmapAHigh = 0x00002001,
    IoBitmapB = 0x00002002,
    IoBitmapBHigh = 0x00002003,
    MsrBitmap = 0x00002004,
    MsrBitmapHigh = 0x00002005,
    VmExitMsrStoreAddr = 0x00002006,
    VmExitMsrStoreAddrHigh = 0x00002007,
    VmExitMsrLoadAddr = 0x00002008,
    VmExitMsrLoadAddrHigh = 0x00002009,
    VmEntryMsrLoadAddr = 0x0000200a,
    VmEntryMsrLoadAddrHigh = 0x0000200b,
    ExecVmcsPointer = 0x0000200c,
    TscOffset = 0x00002010,
    TscOffsetHigh = 0x00002011,
    VirtualApicPageAddr = 0x00002012,
    VirtualApicPageAddrHigh = 0x00002013,
    VmfuncControls = 0x00002018,
    VmfuncControlsHigh = 0x00002019,
    EptPointer = 0x0000201A,
    EptPointerHigh = 0x0000201B,
    EptpList = 0x00002024,
    EptpListHigh = 0x00002025,
    GuestPhysicalAddress = 0x00002400,
    GuestPhysicalAddressHigh = 0x00002401,
    VmcsLinkPointer = 0x00002800,
    VmcsLinkPointerHigh = 0x00002801,
    GuestIa32Debugctl = 0x00002802,
    GuestIa32DebugctlHigh = 0x00002803,
    GuestIa32Efer = 0x00002806,
    HostIa32Pat = 0x00002c00,
    HostIa32Efer = 0x00002c02,
    HostIa32EferHigh = 0x00002c03,
    PinBasedVmExecControls = 0x00004000,
    ProcBasedVmExecControls = 0x00004002,
    ExceptionBitmap = 0x00004004,
    PageFaultErrorCodeMask = 0x00004006,
    PageFaultErrorCodeMatch = 0x00004008,
    Cr3TargetCount = 0x0000400a,
    VmExitControls = 0x0000400c,
    VmExitMsrStoreCount = 0x0000400e,
    VmExitMsrLoadCount = 0x00004010,
    VmEntryControls = 0x00004012,
    VmEntryMsrLoadCount = 0x00004014,
    VmEntryIntrInfoField = 0x00004016,
    VmEntryExceptionErrorCode = 0x00004018,
    VmEntryInstructionLen = 0x0000401a,
    TprThreshold = 0x0000401c,
    ProcBasedVmExecControls2 = 0x0000401e,
    VmInstructionError = 0x00004400,
    VmExitReason = 0x00004402,
    VmExitIntrInfo = 0x00004404,
    VmExitIntrErrorCode = 0x00004406,
    IdtVectoringInfoField = 0x00004408,
    IdtVectoringErrorCode = 0x0000440a,
    VmExitInstructionLen = 0x0000440c,
    VmxInstructionInfo = 0x0000440e,
    GuestEsLimit = 0x00004800,
    GuestCsLimit = 0x00004802,
    GuestSsLimit = 0x00004804,
    GuestDsLimit = 0x00004806,
    GuestFsLimit = 0x00004808,
    GuestGsLimit = 0x0000480a,
    GuestLdtrLimit = 0x0000480c,
    GuestTrLimit = 0x0000480e,
    GuestGdtrLimit = 0x00004810,
    GuestIdtrLimit = 0x00004812,
    GuestEsAccessRights = 0x00004814,
    GuestCsAccessRights = 0x00004816,
    GuestSsAccessRights = 0x00004818,
    GuestDsAccessRights = 0x0000481a,
    GuestFsAccessRights = 0x0000481c,
    GuestGsAccessRights = 0x0000481e,
    GuestLdtrAccessRights = 0x00004820,
    GuestTrAccessRights = 0x00004822,
    GuestInterruptibilityState = 0x00004824,
    GuestActivityState = 0x00004826,
    GuestSmBase = 0x00004828,
    GuestIa32SysenterCs = 0x0000482A,
    HostIa32SysenterCs = 0x00004c00,
    Cr0GuestHostMask = 0x00006000,
    Cr4GuestHostMask = 0x00006002,
    Cr0ReadShadow = 0x00006004,
    Cr4ReadShadow = 0x00006006,
    Cr3TargetValue0 = 0x00006008,
    Cr3TargetValue1 = 0x0000600a,
    Cr3TargetValue2 = 0x0000600c,
    Cr3TargetValue3 = 0x0000600e,
    ExitQualification = 0x00006400,
    GuestLinearAddress = 0x0000640a,
    GuestCr0 = 0x00006800,
    GuestCr3 = 0x00006802,
    GuestCr4 = 0x00006804,
    GuestEsBase = 0x00006806,
    GuestCsBase = 0x00006808,
    GuestSsBase = 0x0000680a,
    GuestDsBase = 0x0000680c,
    GuestFsBase = 0x0000680e,
    GuestGsBase = 0x00006810,
    GuestLdtrBase = 0x00006812,
    GuestTrBase = 0x00006814,
    GuestGdtrBase = 0x00006816,
    GuestIdtrBase = 0x00006818,
    GuestDr7 = 0x0000681a,
    GuestRsp = 0x0000681c,
    GuestRip = 0x0000681e,
    GuestRflags = 0x00006820,
    GuestPendingDbgExceptions = 0x00006822,
    GuestSysenterEsp = 0x00006824,
    GuestSysenterEip = 0x00006826,
    HostCr0 = 0x00006c00,
    HostCr3 = 0x00006c02,
    HostCr4 = 0x00006c04,
    HostFsBase = 0x00006c06,
    HostGsBase = 0x00006c08,
    HostTrBase = 0x00006c0a,
    HostGdtrBase = 0x00006c0c,
    HostIdtrBase = 0x00006c0e,
    HostIa32SysenterEsp = 0x00006c10,
    HostIa32SysenterEip = 0x00006c12,
    HostRsp = 0x00006c14,
    HostRip = 0x00006c16,
}

const VMCS_PROC_BASED_VMEXEC_CTLS_HLTEXIT: u64 = 1 << 7;
const VMCS_PROC_BASED_VMEXEC_CTLS_ACTIVE_SECOND_CTLS: u64 = 1 << 31;

const VMCS_PROC_BASED_VMEXEC_CTLS2_ENABLE_EPT: u64 = 1 << 1;

const VMCS_VMEXIT_CTLS_HOST_ADDR_SPACE_SIZE: u64 = 1 << 9;

const VMCS_VMENTRY_CTLS_IA32E_MODE_GUEST: u64 = 1 << 9;
