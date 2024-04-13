#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use burritos::{exit_qemu, serial_print, serial_println, QemuExitCode};
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

fn should_fail() {
    serial_println!("Should panic test in routine!");
    assert_eq!(0, 1);
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
        serial_println!("[test did not panic]");
        exit_qemu(QemuExitCode::Failed);
    }
    exit_qemu(QemuExitCode::Success);
}

//harness = false
//mono-test integration test crates are run as an executable rather than #[test_case] functions

//#[test_case]
//fn should_panic_test() {
//    serial_print!("should_panic::should_panic_test...\t");
//    assert_eq!(0, 1);
//}

//#[test_case]
//fn should_panic_test_1() {
//    serial_print!("should_panic::should_panic_test_1...\t");
//    assert_eq!(0, 1);
//}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
