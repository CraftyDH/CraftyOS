//* CONSTANTS
const BUFFER_HEIGHT: usize = 25; // 25 charaters tall
const BUFFER_WIDTH: usize = 80; // 80 character wide

//* Use statements
use core::fmt; // So we can implement a formater
use spin::Mutex; // So that we can spinlock the WRITER.
use volatile::Volatile; // To stop compiler optimising away writes

//* All the vga text buffer colours
#[allow(dead_code)] // Ignore if some colours aren't used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
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
pub struct ColourCode(u8);

impl ColourCode {
    pub fn from_fg(foreground: Colour) -> ColourCode {
        ColourCode::new(foreground, Colour::Black)
    }
    pub fn new(foreground: Colour, background: Colour) -> ColourCode {
        // The first 4 bits are background and the last 4 are foreground
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    colour_code: ColourCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

struct Pos {
    x: usize,
    y: usize,
}

pub struct Writer {
    pos: Pos,
    colour_code: ColourCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                // Check if we have written to the end of the line, if so create a new line
                if self.pos.x >= BUFFER_WIDTH {
                    self.new_line();
                }

                let col = self.pos.x;
                let row = self.pos.y;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    colour_code: self.colour_code,
                });

                self.pos.x += 1;
            }
        }
    }

    pub fn write_str(&mut self, string: &str) {
        for byte in string.bytes() {
            self.write_byte(byte);
        }
    }

    fn new_line(&mut self) {
        self.pos.y += 1;
        self.pos.x = 0;
        if self.pos.y >= BUFFER_HEIGHT {
            // Interate over the height and BUFFER_WIDTH
            // Then read the character and write it a line up
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let chr = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(chr);
                }
            }
            // Clear the bottom row and set coloum pos back to the beginning
            self.clear_row(BUFFER_HEIGHT - 1);
            self.pos.y -= 1;
        }
    }
    fn clear_row(&mut self, row: usize) {
        // Set every character to a space
        let blank = ScreenChar {
            ascii_character: b' ',
            colour_code: self.colour_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn set_colour(&mut self, colour: ColourCode) {
        self.colour_code = colour;
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        pos: Pos { x: 0, y: 0 },
        colour_code: ColourCode::new(Colour::White, Colour::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

//* Macros

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
