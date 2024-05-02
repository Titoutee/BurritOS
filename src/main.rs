// Each crate root has to define its own panic handler routine in the test and/or not(test) cases
// as well as its _start routine

// Target triple: <arch><sub>-<vendor>-<sys>-<env>

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(burritos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use burritos::memory;
use burritos::println;
use x86_64::structures::paging::Page;
use core::panic::PanicInfo;
use bootloader::{entry_point, BootInfo};

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
    use x86_64::{VirtAddr, structures::paging::Page};
    println!("Hello world!");
    let ptr = 0x2031b2 as *mut u8; // Code segment (on my machine, may not work on another machine)
    burritos::init();

    let x = unsafe { *ptr }; // Reading from the cs
    println!("read succeeded");
    //unsafe {
    //    *ptr = 42; // Should throw a PAGE_FAULT exception as permissions are violated
    //}
    //println!("write succeeded"); // Writing to the cs

    println!("{:X}", boot_info.physical_memory_offset);
    // Retrieve physical memory offset
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = memory::EmptyFrameAllocator;

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    burritos::hlt_loop();
}
