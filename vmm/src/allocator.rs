use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(head: usize, len: usize) {
    ALLOCATOR.lock().init(head, len);
}
