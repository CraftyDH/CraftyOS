//* CONSTANTS
const BUFFER_HEIGHT: usize = 25; // 25 charaters tall
const BUFFER_WIDTH: usize = 80; // 80 character wide

//* Submouldes
pub mod colour;
pub mod writer;

//* Use statements
use colour::ColourCode;
use core::fmt; // So we can implement a formater
use writer::WRITER;

//* VGA Buffer macros
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
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

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _colour(colour: ColourCode) {
    WRITER.lock().set_colour(colour);
}
