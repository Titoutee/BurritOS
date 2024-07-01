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
use burritos::allocator::linked_list::LinkedListAlloc;
use bootloader::{entry_point, BootInfo};
use burritos::{hlt_loop, memory};
use burritos::memory::BootInfoFrameAllocator;
use burritos::println;
use burritos::task::{Task, task_executor::Executor, simple_executor::SimpleExecutor};
use core::panic::PanicInfo;
use core::marker::PhantomPinned;
use core::pin::Pin;
use burritos::task::keyboard::print_keypresses;

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
    let ptr = 0x2031b2 as *mut u8; // Code segment (on my insatance, may not work on another)
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
    println!("Before dealloc!");
    println!("vec at {:p}", vec.as_slice());
    println!("After dealloc!");
    
    let ref_counted = Rc::new(vec![1, 2, 3]);
    let cloned_ref = ref_counted.clone();
    assert_eq!(2, Rc::strong_count(&cloned_ref));
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_ref)
    );
    core::mem::drop(ref_counted); // Manual drop
    assert_eq!(1, Rc::strong_count(&cloned_ref));
    println!(
        "current reference count is now {}",
        Rc::strong_count(&cloned_ref)
    );

    // To demo how mem::replace() can break the pinning mechanism of heap allocation
    struct SelfRef {
        self_ptr: *const SelfRef,
    }
    let mut heap_value = Box::new(SelfRef {
        self_ptr: 0 as *const SelfRef,
    });
    let ptr = &*heap_value as *const SelfRef;
    heap_value.self_ptr = ptr;
    assert_eq!(&*heap_value as *const SelfRef as usize, heap_value.self_ptr as usize);
    println!("heap value at {:p}", heap_value);
    println!("internal reference : {:p}", heap_value.self_ptr);
    // Everything works, BUT...
    // std::mem::replace(&mut *heap_value, SelfRef { self_ptr: 0 as *const SelfRef });
    // &stack_value != stack_value.self_ptr

    // Rather use pinning instead
    struct SelfRefPinned {
        self_ptr: *const SelfRefPinned,
        _pin: PhantomPinned, // Opt-out of Unpin
    }
    let mut heap_value = Box::pin(SelfRefPinned {
        self_ptr: 0 as *const SelfRefPinned,
        _pin: PhantomPinned,
    });
    let ptr = &*heap_value as *const SelfRefPinned;
    unsafe {
        let mut_ref = Pin::as_mut(&mut heap_value);
        Pin::get_unchecked_mut(mut_ref).self_ptr = ptr;
    }

    assert_eq!(&*heap_value as *const SelfRefPinned as usize, heap_value.self_ptr as usize);
    println!("heap value at {:p}", heap_value);
    println!("internal reference : {:p}", heap_value.self_ptr);

    // Async executor
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(print_keypresses()));
    executor.run();
    hlt_loop();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}