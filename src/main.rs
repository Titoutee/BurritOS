// Each crate root has to define its own panic handler routine in the test and/or not(test) cases
// as well as its _start routine

// Target triple: <arch><sub>-<vendor>-<sys>-<env>

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(burritos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

use bootloader::{entry_point, BootInfo};
use burritos::memory;
use burritos::memory::BootInfoFrameAllocator;
use burritos::println;
use core::panic::PanicInfo;

entry_point!(kernel_main);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    loop {} // !
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    burritos::test_panic_handler(info);
}

// Main
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use burritos::allocator;
    use x86_64::{structures::paging::Page, VirtAddr};

    // Init
    println!("Hello world!");
    let ptr = 0x2031b2 as *mut u8; // Code segment (on my machine, may not work on another machine)
    burritos::init();

    // Paging
    let x = unsafe { *ptr }; // Reading from the cs
    println!("read succeeded");

    //unsafe {
    //    *ptr = 42; // Should throw a PAGE_FAULT exception as permissions are violated
    //}
    //println!("write succeeded"); // Writing to the cs

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // Mapper used to create new mappings (can induce the creation of new page table pages of level 4, 3 or 2)
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

    // Alloc
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    let heap_val = Box::new(41);
    println!("heap_value at {:p}", heap_val);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    let ref_counted = Rc::new(vec![1, 2, 3]);
    let cloned_ref = ref_counted.clone();
    assert_eq!(2, Rc::strong_count(&cloned_ref));
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_ref)
    );
    core::mem::drop(ref_counted);
    assert_eq!(1, Rc::strong_count(&cloned_ref));
    println!(
        "current reference count is now {}",
        Rc::strong_count(&cloned_ref)
    );

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    burritos::hlt_loop();
}
