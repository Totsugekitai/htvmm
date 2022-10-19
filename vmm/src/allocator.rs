use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(heap: usize, len: usize) {
    ALLOCATOR.lock().init(heap as *mut u8, len);
}
