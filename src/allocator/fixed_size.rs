// Fixed-size allocation cuts the heap in several fixed-size slabs, joined as a free linked list under a common
// node size parameter. This allocator provides slab sizes of 8, 16, 32, 64, 128, 256, 512, 1024 and 2048 bytes.
//The use of powers of 2 permits additional slab spawning  (in case heap runs out of slabs of a prticular size),
// so that splitting a bigger free block into two smaller is feasible. Ths permits to avoid using a fallback allocator
// in case of a lack of free slabs. However, this fallback solution maybe used for large block allocation, which are
// rare among kernel allocation.
// The fixed-size pattern provides allocation and deallocation methods which are O(1), as no traversal
// of the free-list is needed.

use core::{alloc::{GlobalAlloc, Layout}, ptr::{self, NonNull}, mem};

use super::Locked;

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
// Slab size does not go under 8B, because at least a 64-bit pointer must fit into it.

struct Node {
    next: Option<&'static mut Node>,
    // No size, because all nodes under a common size per list
}

fn list_index(layout: &Layout) -> Option<usize> {
    let block_size = layout.size().max(layout.align()); // The block size is its alignment
    BLOCK_SIZES.iter().position(|&fb_size| fb_size >= block_size)
}

pub struct FixedSizeAlloc {
    list_heads: [Option<&'static mut Node>; BLOCK_SIZES.len()],
    fallback_alloc: linked_list_allocator::Heap,
}

impl FixedSizeAlloc {
    pub const fn new() -> Self {
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
}

unsafe impl GlobalAlloc for Locked<FixedSizeAlloc> {
    /// Retrieves a (potential) node from the corresponding size linked list, so that is is
    /// popped from the front of the free list. If the chosen free list is empty (which is the initial case)
    /// blocks are lazily allocated for that list, and remain maintained by the fallback even when `dealloc`
    /// is called. This way, the fixed size allocator is built upon its fallback allocator, taking profit of the latter's
    /// allocation. 
    /// One consequence of this is that fixed-size deallocation is "virtual", in the sense that the actual blocks are not
    /// freed other than from the free-list. The fallback allocator does not free the blocks at each deallocation.
    /// 
    /// One exception is if dealloc is used in another context than on one fixed-size linked-list. In this case, the fallback
    /// allocator physically deallocates the given slab.
    /// 
    /// **Performance overhead**: initial allocations may suffer from low performance due to the systematic use of the fallback
    /// allocator.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(head) => {
                        allocator.list_heads[index] = head.next.take();
                        head as *mut Node as *mut u8
                    }
                    None => { // lazy alloc
                        let block_size = BLOCK_SIZES[index];
                        let block_align = block_size;
                        let alloc_layout = Layout::from_size_align(block_size, block_align).unwrap();
                        allocator.fallback_alloc(alloc_layout)
                    }
                }
            } 
            None => allocator.fallback_alloc(layout) // Other size alloc
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => { // virtual dealloc
                let new_node = Node {
                    next: allocator.list_heads[index].take(),
                };
                // verify that block has size and alignment required for storing node
                assert!(mem::size_of::<Node>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<Node>() <= BLOCK_SIZES[index]);
                // store node, thus freeing this area
                let new_node_ptr = ptr as *mut Node;
                new_node_ptr.write(new_node);
                // update the head
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => { // Allocation was not made in a fixed-size compliant manner, but rather by the fallback allocator
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback_alloc.deallocate(ptr, layout);
            }
        }
    }
}