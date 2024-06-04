#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(burritos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use burritos::{println, serial_println};
use core::panic::PanicInfo;
use core::ptr::{null_mut, write};
use burritos::allocator::ALLOCATOR;
use core::alloc::{GlobalAlloc, Layout};
use alloc::boxed::Box;
use alloc::vec::Vec;
use burritos::allocator::HEAP_SIZE;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use burritos::allocator;
    use burritos::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    burritos::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe {memory::init(phys_mem_offset)};
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    
    test_main();
    loop{}
}

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

// Large alloc
#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::with_capacity(1000); // Permits faster allocation
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn manual_alloc() {
    let layout = Layout::new::<Vec<u8>>();
    unsafe {
        let a = ALLOCATOR.alloc(layout);
        assert_ne!(a, null_mut());
        ALLOCATOR.dealloc(a, layout);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    burritos::test_panic_handler(info)
}