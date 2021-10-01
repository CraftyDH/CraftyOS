//* Use statements
use super::colour::{Colour, ColourCode};
use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
use alloc::format;
use core::convert::TryInto;
use core::fmt;
// So we can implement a formater
use spin::Mutex; // So that we can spinlock the WRITER.
use volatile::Volatile;
use x86_64::instructions::port::Port; // To stop compiler optimising away writes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    colour_code: ColourCode,
}

#[repr(transparent)]
pub struct Buffer {
    pub chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

struct Pos {
    x: usize,
    y: usize,
}

pub struct Writer {
    pos: Pos,
    colour_code: ColourCode,
    pub buffer: &'static mut Buffer,
    flipped: Pos,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8, cursor: bool) {
        match byte {
            b'\n' => self.new_line(),
            b'\x08' => {
                // Backspace
                let mut x = self.pos.x;
                let mut y = self.pos.y;

                // If at beginning of line go up to previous line
                if x == 0 {
                    // If on the top line just stay at pos (0, 0)
                    if y != 0 {
                        y -= 1;
                        x = BUFFER_WIDTH
                    }
                } else {
                    x -= 1;
                }

                // Update new positions
                self.pos.x = x;
                self.pos.y = y;

                // Write over previous character with a space
                // We will update cursor soon 
                self.write_byte(b' ', false);

                // It incremented again so go backwords again
                self.pos.x = x;
                self.pos.y = y;

                // Update cursor
                // let pos = y * BUFFER_WIDTH + x;
                self.update_cursor();
            },
            byte => {
                // Check if we have written to the end of the line, if so create a new line
                if self.pos.x >= BUFFER_WIDTH {
                    self.new_line();
                }

                let col = self.pos.x;
                let row = self.pos.y;

                let mut colour_code = self.colour_code;

                // If cursor is here flip bit
                if col == self.flipped.x && row == self.flipped.y {
                    colour_code.flip();
                }

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    colour_code: colour_code,
                });

                self.pos.x += 1;

                if cursor {
                    self.update_cursor();
                }
            }
        }
    }

    fn update_cursor(&self) {
        let pos = self.pos.y * BUFFER_WIDTH + self.pos.x;
        let mut a = Port::<u8>::new(0x3D4);
        let mut b = Port::<u8>::new(0x3D5);
        unsafe {
            a.write(0x0F);
            b.write((pos & 0xFF).try_into().unwrap());
            a.write(0x0E);
            b.write((pos >> 8 & 0xFF).try_into().unwrap());
        }
    }

    pub fn write_str(&mut self, string: &str) {
        for byte in string.bytes() {
            self.write_byte(byte, false);
        }
        self.update_cursor();
    }

    fn new_line(&mut self) {
        self.pos.y += 1;
        self.pos.x = 0;

        // Flip cursor so it doesn't get copied down
        self.flip_bit(self.flipped.x, self.flipped.y);
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
        // Set cursor again
        self.flip_bit(self.flipped.x, self.flipped.y);

        // Update cursor
        self.update_cursor();
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

    pub fn set_pos(&mut self, x: usize, y: usize) {
        self.pos = Pos { x, y }
    }

    pub fn flip_bit(&mut self, x: usize, y: usize) {
        // if self.flipped.x != x || self.flipped.y != y {
            let mut origin = self.buffer.chars[y][x].read();
            origin.colour_code.flip();

            self.flipped.x = x;
            self.flipped.y = y;

            self.buffer.chars[y][x].write(origin);
        // }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = {
        let mut writer = Writer {
            pos: Pos { x: 0, y: 0 },
            colour_code: ColourCode::new(Colour::White, Colour::Black),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
            flipped: Pos { x: 0, y: 0 }
        };

        // Init the entire buffer with default colour and spaces
        for _ in 0..BUFFER_HEIGHT * 2 {
            writer.new_line()
        }

        writer.pos = Pos { x: 0, y: 0 };

        Mutex::new( writer)
    };
}
