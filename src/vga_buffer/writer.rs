//* Use statements
use super::colour::{Colour, ColourCode};
use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
use core::fmt; // So we can implement a formater
use spin::Mutex; // So that we can spinlock the WRITER.
use volatile::Volatile; // To stop compiler optimising away writes

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
