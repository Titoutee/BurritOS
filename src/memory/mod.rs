use core::fmt::Display;

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{structures::paging::PageTable, VirtAddr};
use x86_64::{
    PhysAddr,
    structures::paging::{Page, PhysFrame, Mapper, Size4KiB, FrameAllocator, mapper::OffsetPageTable}
};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    // access to physical address of the lvl 4 page table
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    &mut *(virt.as_mut_ptr() as *mut PageTable) // unsafe
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Creates an iterator over the usable frames specified in the memory map
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

impl Display for BootInfoFrameAllocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.memory_map)
    }
}

/// Creates an example mapping for the given page to frame `0xb8000`.
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000)); // Find the physical frame containing addr 0xb8000
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let mappping = unsafe {
        // FIXME: this is not safe, we do it only for testing
        // Indeed, the mapping may be already present, leading to UB
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    mappping.expect("map_to failed").flush();
}

//
//use x86_64::PhysAddr;
//
// Translates the given virtual address to the mapped physical address, or
// `None` if the address is not mapped.
//
// This function is unsafe because the caller must guarantee that the
// complete physical memory is mapped to virtual memory at the passed
// `physical_memory_offset`.
//pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
//    translate_addr_inner(addr, physical_memory_offset)
//}

// Private function that is called by `translate_addr`.
//
// This function is safe to limit the scope of `unsafe` because Rust treats
// the whole body of unsafe functions as an unsafe block. This function must
// only be reachable through `unsafe fn` from outside of this module.
//fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
//    use x86_64::registers::control::Cr3;
//    use x86_64::structures::paging::page_table::FrameError;
//
//    // read the active level 4 frame from the CR3 register
//    let (level_4_table_frame, _) = Cr3::read(); //Physical Address
//
//    // Extract the indexes from the virtual address, which index through different page table levels
//    let table_indexes = [
//        addr.p4_index(),
//        addr.p3_index(),
//        addr.p2_index(),
//        addr.p1_index(),
//    ];
//    let mut frame = level_4_table_frame; // Starting frame
//
//    // traverse the multi-level page table
//    for index in table_indexes {
//        // convert the frame into a page table reference
//        let virt = physical_memory_offset + frame.start_address().as_u64();
//        let table_ptr: *const PageTable = virt.as_ptr();
//        let table = unsafe { &*table_ptr }; // Access the frame through the virtual address
//
//        // read the page table entry and update `frame`
//        let entry = &table[index];
//        frame = match entry.frame() {
//            // Get the PFN out of the PTE
//            Ok(frame) => frame,
//            Err(FrameError::FrameNotPresent) => return None, // The pathway encountered an unallocated or swapped out page
//            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
//        };
//    }
//
//    // calculate the physical address by adding the page offset
//    // `frame` should contain the final mapped frame after level 1 dereferencing
//    Some(frame.start_address() + u64::from(addr.page_offset()))
//}
//