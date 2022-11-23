use crate::{
    arch::intel::vmx::{vmclear, vmptrld},
    cpu::{Gdtr, Tr},
    BOOT_ARGS,
};
use alloc::alloc::alloc;
use core::{alloc::Layout, ptr, slice};
use x86_64::{
    registers::{
        model_specific::Msr,
        segmentation::{Segment, CS, DS, ES, FS, GS, SS},
    },
    PhysAddr,
};

use super::vmx::vmwrite;

pub struct VmcsRegion(*mut u8);

impl VmcsRegion {
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align(4096, 4096).unwrap();
        let region = alloc(layout);
        let region_slice = slice::from_raw_parts_mut(region, 4096);
        region_slice.fill(0);

        let ia32_vmx_basic = Msr::new(0x480).read(); // MSR_IA32_VMX_BASIC
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

    fn write(&mut self, field: VmcsField, val: u64) {
        unsafe {
            vmwrite(field, val);
        }
    }

    pub fn setup(&mut self) {
        self.setup_guest_state_area();
        self.setup_host_state_area();
        self.setup_vm_execution_control_fields();
        self.setup_vm_exit_control_fields();
        self.setup_vm_entry_control_fields();
        self.setup_vm_exit_infomation_fields();
    }

    fn setup_guest_state_area(&mut self) {
        let debugctl = unsafe { Msr::new(0x1d9).read() };
        let debugctl_low = debugctl & 0xffff_ffff;
        let debugctl_high = (debugctl >> 32) & 0xffff_ffff;
        self.write(VmcsField::GuestIa32Debugctl, debugctl_low);
        self.write(VmcsField::GuestIa32DebugctlHigh, debugctl_high);

        self.write(VmcsField::VmcsLinkPointer, !0u64);
    }

    fn setup_host_state_area(&mut self) {
        let cs = CS::get_reg();
        let ds = DS::get_reg();
        let es = ES::get_reg();
        let fs = FS::get_reg();
        let gs = GS::get_reg();
        let ss = SS::get_reg();
        self.write(VmcsField::HostCsSelector, cs.0 as u64);
        self.write(VmcsField::HostDsSelector, ds.0 as u64);
        self.write(VmcsField::HostEsSelector, es.0 as u64);
        self.write(VmcsField::HostFsSelector, fs.0 as u64);
        self.write(VmcsField::HostGsSelector, gs.0 as u64);
        self.write(VmcsField::HostSsSelector, ss.0 as u64);

        let gdtr = Gdtr::get_reg();
        let tr = Tr::get_reg();
        self.write(VmcsField::HostTrSelector, tr.as_u64());
        self.write(VmcsField::HostGdtrBase, gdtr.base());
    }

    fn setup_vm_execution_control_fields(&mut self) {
        self.write(VmcsField::TscOffset, 0);
        self.write(VmcsField::TscOffsetHigh, 0);

        self.write(VmcsField::PageFaultErrorCodeMask, 0);
        self.write(VmcsField::PageFaultErrorCodeMatch, 0);
    }

    fn setup_vm_exit_control_fields(&mut self) {
        self.write(VmcsField::VmExitMsrLoadCount, 0);
        self.write(VmcsField::VmExitMsrStoreCount, 0);
    }

    fn setup_vm_entry_control_fields(&mut self) {
        self.write(VmcsField::VmEntryMsrLoadCount, 0);
        self.write(VmcsField::VmEntryIntrInfoField, 0);
    }

    fn setup_vm_exit_infomation_fields(&mut self) {}
}

#[repr(u32)]
pub enum VmcsField {
    GuestEsSelector = 0x00000800,
    GuestCsSelector = 0x00000802,
    GuestGsSelector = 0x00000804,
    GuestDsSelector = 0x00000806,
    GuestFsSelector = 0x00000808,
    GsSelector = 0x0000080a,
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
    PinBasedVmExecControl = 0x00004000,
    CpuBasedVmExecControl = 0x00004002,
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
    SecondaryVmExecControl = 0x0000401e,
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
    GuestEsArBytes = 0x00004814,
    GuestCsArBytes = 0x00004816,
    GuestSsArBytes = 0x00004818,
    GuestDsArBytes = 0x0000481a,
    GuestFsArBytes = 0x0000481c,
    GuestGsArBytes = 0x0000481e,
    GuestLdtrArBytes = 0x00004820,
    GuestTrArBytes = 0x00004822,
    GuestInterruptibilityInfo = 0x00004824,
    GuestActivityState = 0x00004826,
    GuestSmBase = 0x00004828,
    GuestSysenterCs = 0x0000482A,
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
