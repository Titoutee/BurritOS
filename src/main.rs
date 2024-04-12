#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod exit;
mod serial;
mod vga;
// Target triple: <arch><sub>-<vendor>-<sys>-<env>

use core::panic::PanicInfo;
use exit::{exit_qemu, QemuExitCode};

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    loop {} // !
}

#[test_case]
fn trivial_assertion() {
    serial_print!("trivial assertion... ");
    assert_eq!(1, 1);
    serial_println!("[ok]");
}

// Main
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello world!");
    #[cfg(test)]
    test_main();
    println!("From Titoutee's kernel!\n");
    println!("Hello Universe!\n");

    loop {}
}
