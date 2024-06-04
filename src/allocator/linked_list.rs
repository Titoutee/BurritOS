use crate::{println, serial_println};

use super::{align_up, Locked};
use core::{alloc::{GlobalAlloc, Layout}, mem, ptr};

struct Node {
    size: usize,
    next: Option<&'static mut Node>,
}

impl Node {
    const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAlloc {
    head: Node,
}

impl LinkedListAlloc {
    pub const fn new() -> Self {
        Self {
            head: Node::new(0), // Always the tail of the freelist
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.add_free_region(heap_start, heap_end);
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `Node`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Node>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Node>());
        (size, layout.align())
    }

    /// Adds the given memory region so that the freelist remains sorted
    /// DO NOT USE
    //unsafe fn _add_free_region_merged(&mut self, addr: usize, size: usize) {
    //    // Alignment checks
    //    assert_eq!(align_up(addr, mem::align_of::<Node>()), addr);
    //    assert!(size >= mem::size_of::<Node>()); // Is there enough space free to allocate a Node on it?

    //    let current = &mut self.head;
    //    
    //    // Searches for a sorted place for the new free space
    //    while let Some(ref free_region) = current.next {
    //        if free_region.start_addr() > addr {
    //            break;
    //        } 
    //        // Thus leaves `current` as being the block before the new free block
    //    }

    //    if current.end_addr() == addr {
    //        serial_println!("WOW, some blocks were merged at addr {}", addr);
    //        current.size += size;
    //        // TODO: impl merging with following block also
    //    }
    //    else {
    //        // We create a new node, not mergeable with any other
    //        let mut node = Node::new(size);
    //        node.next = current.next.take(); // Copy + reset to None
    //        let node_ptr = addr as *mut Node;
    //        node_ptr.write(node); // Writes the actual free list node
    //        current.next = Some(&mut *node_ptr); // Current now points to the new free zone
    //    }        
    //}

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed region is capable of holding ListNode
        assert_eq!(align_up(addr, mem::align_of::<Node>()), addr);
        assert!(size >= mem::size_of::<Node>());

        // create a new list node and append it at the start of the list
        let mut node = Node::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut Node;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    /// Looks for a free region with the given size and alignment and removes
    /// it from the list.
    /// 
    /// Returns a tuple of the list node and the start address of the allocation.
    /// Wrapper around `alloc_from_region`, which is here used to try allocation until a slab 
    /// is able to receive the to-be-allocated region.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut Node, usize)> {
        let mut current = &mut self.head;
        
        // Goes through the whole freelist
        while let Some(ref mut region) = current.next {
    
            // If region suitable for allocation given size and align constraints,
            // remove the linked list node at that region, setting `next` to its following node
            if let Some(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take(); // Takes next from the elected node
                let ret = Some((current.next.take().unwrap(), alloc_start)); // Takes the suitable region
                current.next = next; // Close the gap: current's next = region's next, while region = current's next
                return ret;
            } else {
                current = current.next.as_mut().unwrap(); // Not elected? Just continue through the l-l.
            }
        }
        None
    }

    /// Allocates a region currently populated by a Node instance and excess memory
    /// given a region and a size of allocation as well as alignment constraints.
    /// The allocated region should either fit perfectly, thus leaving no excess free space,
    /// or leave at least `size_of::<Node>()` bytes of excess for the creation of a new Node.
    fn alloc_from_region(region: &Node, size: usize, align: usize) -> Option<usize> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size)?;

        if alloc_end > region.end_addr() {
            return None;
        }

        let excess_size = region.end_addr()-alloc_end;

        // Either allocation fits perfectly in the slab, or if not, a Node should fit.
        // If not the case, not possible to allocate.
        if excess_size > 0 && excess_size < mem::size_of::<Node>() {
            return None;
        }
        Some(alloc_start)
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAlloc> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let (size, align) = LinkedListAlloc::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            // These are calculated again to check whether it allocates or not a new Node for the potential
            // excess memory.
            // Indeed, as `find_region` succeeded at this point, it is impossible that 0<=excess<sizeof(Node), 
            // so no security check occurs.
            let alloc_end = alloc_start.checked_add(size).expect("overflow in alloc_end calculation");
            let excess_size = region.end_addr() - alloc_end;

            // Allocates a new free region if possible
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size); // Add the (possibly tiny) region to free list
            }
            // Trolling (but for info!)
            else if excess_size == mem::size_of::<Node>() {
                println!("Excess was perfectly fit for a new Node! Hooray!"); // To be removed :)
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut() // No free region was found :(
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // perform layout adjustments
        let (size, _) = LinkedListAlloc::size_align(layout);
        // Adds this zone to the free list
        self.lock().add_free_region(ptr as usize, size);
        println!("Object at addr {} was dropped!", ptr as usize);
        // Now free! (not cleared)
    }
}
