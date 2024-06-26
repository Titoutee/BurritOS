//! Behaviour around the VGA text mode, providing a volatile safe-ish writer to the mapped VGA buffer

use crate::serial_println;
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// Evaluate static at runtime, so no need for const functions' calls
lazy_static! {
    // Safely shared across threads Writer
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }, // Only unsafe operation
    });
}

// Some cumbersome macro definitions to pseudo-implement basic output mecanisms
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        // This is valid since the repr(u8) of Color variants is 4-bit, so no loss occurs when shifting
        ColorCode((background as u8) << 4 | (foreground as u8)) // Foreground: bits 8-11 (first) / Background: bits 12-14 (second)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // C field ordering is necessary
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// Buffer for VGA outputs
#[repr(transparent)]
struct Buffer {
    // Volatile data: futureproof against rustc optimizations to remove unidirectionnal I/O operations
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// The actual writing structure,
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                // Write the volatile ScreenChar instance, so that no optimization may swipe out these one-sided I/O operations
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    // "Code page 437" characters (UTF-8 unrecognized, and multibyte characters not printed)
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let space_char = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(space_char);
        }
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[allow(dead_code)]
pub fn print_smthg() {
    WRITER.lock().write_byte(b'H');
    WRITER.lock().write_byte(b'\n');
    write!(WRITER.lock(), "The numbers are {} and {}", 42, 1.0 / 3.0).unwrap();
}

#[test_case]
fn println_test() {
    println!("Hello World!");
}

#[test_case]
fn serial_test() {
    serial_println!("Hellow World!");
}

#[test_case]
fn println_output_test() {
    use core::writeln;
    use x86_64::instructions::interrupts;
    let s = "Some test string that fits on a single line";

    // A write is made atomic
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read(); //prinltn!
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    })
}

#[test_case]
fn long_lines_test() {
    let s = "Some test string that does not fit on a single line because it is made up of many characters";

    //serial_println!("{}", s.len());
    println!("{}", s);
}

#[test_case]
fn test_println_output() {
    let s = "Some test string that fits on a single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}

#[test_case]
fn long_lines_wrapping_test() {
    let s = "Some test string that does not fit on a single line because it is made up of many characters";

    //serial_println!("{}", s.len());
    println!("{}", s);

    let line_slice = &s[..BUFFER_WIDTH];
    for (i, c) in line_slice.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 3][i].read(); //println!
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}
