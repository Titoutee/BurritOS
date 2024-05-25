use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

// UART devices are used here through their memory-mapped interface to bring in minimal
// support for serial communication

lazy_static! {
    // Spinlock to ensure cross-thread safety
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

// Some cumbersome macro pseudo-implementation
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    // A write is made atomic
    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args) // SerialPort natively implements fmt::Write
            .expect("Printing to serial failed");
    })
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*)); // Expands to a call to `_print`
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
