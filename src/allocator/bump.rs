
use alloc::alloc::{GlobalAlloc, Layout};
use super::{Locked, align_up};
use core::ptr;

pub struct BumpAlloc {
    heap_start: usize, // Generic heap infos
    heap_end: usize, // Inclusive
    next: usize, // Next block to be allocated
    allocations: usize // Number of allocated entities
}

impl BumpAlloc {
    // Empty BumpAllocator
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0
        }
    }
    /// Initializes the bump allocator with the given heap bounds
    /// 
    /// Must only be called ONCE during the lifespan of the allocator
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) { // Parameters are hardcoded in allocator/mod.rs
        self.heap_start = heap_start;
        self.heap_end =  heap_start + heap_size; // Exclusive end
        self.next = heap_start; // Alloc default is to point to the first block in the heap, which is logically not allocated
    }
}

unsafe impl GlobalAlloc for Locked<BumpAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump_alloc = self.lock();

        let alloc_start = align_up(bump_alloc.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump_alloc.heap_end {
            ptr::null_mut() // Alloc error
        }

        else {
            bump_alloc.next = alloc_end;
            bump_alloc.allocations += 1;
            alloc_start as *mut u8 // Reurn the allocated slab
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump_alloc = self.lock();

        bump_alloc.allocations -= 1;
        if bump_alloc.allocations == 0 {
            bump_alloc.next = bump_alloc.heap_start;
        }
    }
}
