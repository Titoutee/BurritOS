// Each crate root has to define its own panic handler routine in the test and/or not(test) cases
// as well as its _start routine

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(burritos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use burritos::println;
use core::panic::PanicInfo;

#[test_case]
fn test_println() {
    println!("test_println output");
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    burritos::test_panic_handler(info);
}
