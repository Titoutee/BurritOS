
use super::align_up;
use core::mem;

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

pub struct LinkedLIstAlloc {
    head: Node,
}

impl LinkedLIstAlloc {
    pub const fn new() -> Self {
        Self {
            head: Node::new(0),
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.add_free_region(heap_start, heap_end);
    }

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        assert_eq!(align_up(addr, mem::align_of::<Node>()), addr);
        assert!(size >= mem::size_of::<Node>());

        let mut node = Node::new(size);
        node.next = self.head.next.take(); // Copy + reset to None
        self.head.next = Some(&mut *(addr as *mut Node));
    }
}
