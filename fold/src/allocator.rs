use core::ptr;

use linked_list_allocator::LockedHeap;
use rustix::mm;
use rustix::mm::{MapFlags, ProtFlags};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
const HEAP_SIZE_MB: usize = 128;

/// Initializes the global allocator.
///
/// # SAFETY
/// Must be called only once.
pub unsafe fn init_allocator() {
    let heap_size = HEAP_SIZE_MB * 1024 * 1024;
    let prot = ProtFlags::READ | ProtFlags::WRITE;
    let flags = MapFlags::PRIVATE;
    let heap_ptr = mm::mmap_anonymous(ptr::null_mut(), heap_size, prot, flags)
        .expect("Failed to initialize heap");
    ALLOCATOR.lock().init(heap_ptr as _, heap_size as _);
}
