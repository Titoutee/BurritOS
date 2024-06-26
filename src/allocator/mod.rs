use alloc::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use core::ptr::null_mut;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB
    },
    VirtAddr,
};

use linked_list::LinkedListAlloc;
use fixed_size::FixedSizeAlloc;
pub mod bump;
pub mod linked_list;
pub mod fixed_size;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1_024; // 100 KiB

#[global_allocator]
//pub static ALLOCATOR: Locked<LinkedListAlloc> = Locked::new(LinkedListAlloc::new());
pub static ALLOCATOR: Locked<FixedSizeAlloc> = Locked::new(FixedSizeAlloc::new());

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}


/// A wrapper around spin::Mutex to permit trait implementations, in order to bypass immutability implied
/// by GlobalAlloc trait implementation.
pub struct Locked<T> {
    inner: spin::Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(inner: T) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<T> {
        self.inner.lock()
    }
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

//pub unsafe trait GlobalAlloc {
//    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
//    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);
//
//    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { ... }
//    unsafe fn realloc(
//        &self,
//        ptr: *mut u8,
//        layout: Layout,
//        new_size: usize
//    ) -> *mut u8 { ... }
//}