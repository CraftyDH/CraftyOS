//* CONSTANTS
pub const BUFFER_HEIGHT: usize = 25; // 25 charaters tall
pub const BUFFER_WIDTH: usize = 80; // 80 character wide

//* Submouldes
pub mod colour;
pub mod writer;

//* Use statements
use colour::ColourCode;
use core::fmt; // So we can implement a formater
use writer::WRITER;
use x86_64::instructions::interrupts;

//* VGA Buffer macros
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! colour {
    () => {
        $crate::vga_buffer::_colour(ColourCode::new(Colour::White, Colour::Black))
    };
    ($colour: expr) => {
        $crate::vga_buffer::_colour($colour)
    };
}

#[macro_export]
macro_rules! cursor {
    () => {
        $crate::vga_buffer::_cursor(0, 0);
    };
    ($x: expr, $y: expr) => {
        $crate::vga_buffer::_cursor($x, $y);
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    // Prevent race condition by disable interrupts while locking
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _colour(colour: ColourCode) {
    // Prevent race condition by disable interrupts while locking
    interrupts::without_interrupts(|| {
        WRITER.lock().set_colour(colour);
    });
}

#[doc(hidden)]
pub fn _cursor(x: usize, y: usize) {
    // Prevent race condition by disable interrupts while locking
    interrupts::without_interrupts(|| {
        WRITER.lock().set_pos(x, y);
    });
}

//* Tests
#[test_case]
fn test_println() {
    println!("Testing println!");
}

#[test_case]
fn test_println_200() {
    for _ in 0..200 {
        println!("Testing println!");
    }
}

#[test_case]
fn test_println_output() {
    // Set cursor to start
    let s = "Some test string that fits on a single line";

    // No interrupts because any other printing will make the test fail.
    interrupts::without_interrupts(|| {
        cursor!();
        println!("{}", s);
        for (i, c) in s.chars().enumerate() {
            let screen_char = WRITER.lock().buffer.chars[0][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
