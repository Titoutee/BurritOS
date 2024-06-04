// Fixed-size allocation cuts the heap in several fixed-size slabs, joined as a free linked list under a common
// node size parameter. This allocator provides slab sizes of 8, 16, 32, 64, 128, 256, 512, 1024 and 2048 bytes.
//The use of powers of 2 permits additional slab spawning  (in case heap runs out of slabs of a prticular size),
// so that splitting a bigger free block into two smaller is feasible. Ths permits to avoid using a fallback allocator
// in case of a lack of free slabs. However, this fallback solution maybe used for large block allocation, which are
// rare among kernel allocation.
// The fixed-size pattern provides allocation and deallocation methods which are O(1), as no traversal
// of the free-list is needed.

use core::{alloc::{GlobalAlloc, Layout}, ptr};

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
// Slab size does not go under 8B, because at least a 64-bit pointer must fit into it.

struct Node {
    next: Option<&'static mut Node>,
    // No size, because all nodes under a common size per list
}

pub struct FixedSizeAlloc {
    list_heads: [Option<&'static mut Node>; BLOCK_SIZES.len()],
    fallback_alloc: linked_list_allocator::Heap,
}

impl FixedSizeAlloc {
    pub fn new() -> Self {
        const EMPTY:  Option<&'static mut Node> = None;
        FixedSizeAlloc {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_alloc: linked_list_allocator::Heap::empty(),
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_alloc.init(heap_start as *mut u8, heap_size);
    }

    /// Allocates using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_alloc.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    fn list_index(&self, layout: Layout) -> Option<usize> {
        let block_size = layout.size().max(layout.align()); // The block size is its alignment
        BLOCK_SIZES.iter().position(|&fb_size| fb_size >= block_size)
    }
}

unsafe impl GlobalAlloc for FixedSizeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}