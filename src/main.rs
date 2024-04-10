#![no_std]
#![no_main]

// Taregt triple: <arch><sub>-<vendor>-<sys>-<env>

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
