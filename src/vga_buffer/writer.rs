//* Use statements
use super::colour::{self, Colour, ColourCode};
use super::{BUFFER_HEIGHT, BUFFER_WIDTH};
use alloc::vec::Vec;
use alloc::{format, vec};
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
                    // If on the top line just stay at pos (0, 1)
                    if y > 1 {
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
            }
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

    pub fn dump_screen(&mut self) -> Vec<u8> {
        let mut screen: Vec<u8> = Vec::with_capacity(2048);
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let chr = self.buffer.chars[row][col].read();
                screen.push(chr.ascii_character);
                // screen[(row - 1) * BUFFER_HEIGHT * 4 + col] = ;
            }
        }
        screen
    }

    pub fn write_screen(&mut self, mut screen: Vec<u8>) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: screen.pop().unwrap(),
                    colour_code: self.colour_code,
                });
            }
        }
    }

    pub fn write_first_line(&mut self, string: &str, colour: ColourCode) {
        // Get current pos
        let Pos { x, y } = self.pos;
        // Get current colour
        let _colour = self.colour_code;

        // Set colour code to temp colour
        self.colour_code = colour;
        self.set_pos(0, 0);

        // Write the string
        for byte in string.bytes() {
            self.write_byte(byte, false);
        }
        // Set cursor to end of statement
        self.update_cursor();

        // Fill the rest with spaces
        for _ in 0..(BUFFER_WIDTH - string.len()) {
            self.write_byte(b' ', false);
        }
        // Reset POS
        self.set_pos(x, y);
        // Restore colour code
        self.colour_code = _colour;
    }

    fn new_line(&mut self) {
        self.pos.y += 1;
        self.pos.x = 0;

        // Flip cursor so it doesn't get copied down
        self.flip_bit(self.flipped.x, self.flipped.y);

        // Naive implemention which copies each character up, via a loop

        // if self.pos.y >= BUFFER_HEIGHT {
        //     // Interate over the height and BUFFER_WIDTH
        //     // Then read the character and write it a line up
        //     for row in 1..BUFFER_HEIGHT {
        //         for col in 0..BUFFER_WIDTH {
        //             let chr = self.buffer.chars[row][col].read();
        //             self.buffer.chars[row - 1][col].write(chr);
        //         }
        //     }
        //     // Clear the bottom row and set coloum pos back to the beginning
        //     self.clear_row(BUFFER_HEIGHT - 1);
        //     self.pos.y -= 1;
        // }

        // Copy buffer overlappings
        if self.pos.y >= BUFFER_HEIGHT {
            unsafe {
                // The screen buffer uses 16 bits and with a 64 bit pointer.
                // Because of this we can copy 4 characters at a time, at must account for that.
                // 64 / 16 = 4
                // Copy screen up ignoring first line
                core::ptr::copy(
                    (0xb8000 as *const u64).offset((BUFFER_WIDTH / 4 * 2).try_into().unwrap()),
                    (0xb8000 as *mut u64).offset((BUFFER_WIDTH / 4).try_into().unwrap()),
                    (BUFFER_HEIGHT) * (BUFFER_WIDTH) / 4,
                );
            }
            self.pos.y -= 1;
        }
        // Set cursor again
        self.flip_bit(self.flipped.x, self.flipped.y);

        // Update cursor
        self.update_cursor();
    }

    /// Fills screen with spaces with colour of self.colour
    pub fn fill_screen(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            self.fill_row(row);
        }
    }

    pub fn fill_row(&mut self, row: usize) {
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
        self.pos = Pos { x, y };
        self.update_cursor();
    }

    pub fn get_pos(&mut self) -> (usize, usize) {
        (self.pos.x, self.pos.y)
    }

    pub fn flip_bit(&mut self, x: usize, y: usize) {
        // if self.flipped.x != x || self.flipped.y != y {
        let mut origin = self.buffer.chars[y][x].read();
        origin.colour_code.flip();

        self.flipped.x = x;
        self.flipped.y = y;

        self.buffer.chars[y][x].write(origin);
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
            pos: Pos { x: 0, y: 1 },
            colour_code: ColourCode::new(Colour::White, Colour::Black),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
            flipped: Pos { x: 0, y: 0 }
        };

        // First row is special and as such is independant from fill screen
        writer.fill_row(0);
        writer.fill_screen();

        // Init the entire buffer with default colour and spaces
        // for _ in 0..BUFFER_HEIGHT * 2 {
        //     writer.new_line()
        // }

        Mutex::new(writer)
    };
}
