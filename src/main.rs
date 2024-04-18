// Each crate root has to define its own panic handler routine in the test and/or not(test) cases
// as well as its _start routine

// Target triple: <arch><sub>-<vendor>-<sys>-<env>

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(burritos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use burritos::println;
use core::panic::PanicInfo;

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
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello world!");

    burritos::init();
    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    burritos::hlt_loop();
}
