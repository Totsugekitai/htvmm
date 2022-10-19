use super::vmx::vmclear;
use alloc::alloc::alloc;
use core::alloc::Layout;

pub struct VmcsRegion(*mut u8);

impl VmcsRegion {
    pub unsafe fn new() -> Self {
        let layout = Layout::from_size_align_unchecked(4096, 4096);
        Self(alloc(layout))
    }

    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.0
    }

    pub fn clear(&mut self) {
        unsafe {
            vmclear(self);
        }
    }
}
