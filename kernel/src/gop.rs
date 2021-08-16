use core::fmt::Write;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize};
use lazy_static::lazy_static;

fn put_char(
    gop: &mut Gop,
    font: &super::PSF1Font,
    colour: &mut AtomicU32,
    chr: char,
    xoff: usize,
    yoff: usize,
) {
    // let addr = ('A' as usize) * (font.psf1_header.charsize as usize);
    let mut addr = chr as usize * 16;
    let glyphbuf = font.glyph_buffer;

    for y in yoff..(yoff + 16) {
        let glyph = glyphbuf[addr];
        for x in xoff..(xoff + 8) {
            // Fancy math to check if bit is on.
            if (glyph & (0b10_000_000 >> (x - xoff))) > 0 {
                let loc = ((x as usize) + (y as usize * gop.stride)) * 4;
                unsafe {
                    core::ptr::write_volatile(
                        gop.buffer.get_mut().add(loc) as *mut u32,
                        *colour.get_mut(),
                    )
                }
            }
        }
        addr += 1;
    }
}

pub struct Pos {
    pub x: AtomicUsize,
    pub y: AtomicUsize,
}

pub struct Gop {
    pub buffer: AtomicPtr<u8>,
    pub buffer_size: usize,
    pub horizonal: usize,
    pub vertical: usize,
    pub stride: usize,
}

pub struct Writer {
    pub pos: Pos,
    pub gop: Gop,
    pub font: super::PSF1Font<'static>,
    pub colour: AtomicU32,
}

impl Writer {
    pub fn set_gop(&mut self, gop: Gop, font: super::PSF1Font<'static>) {
        self.gop = gop;

        self.font = font;
    }
    pub fn set_colour(&mut self, colour: u32) {
        // Get pointer to then change colour
        *self.colour.get_mut() = colour;
    }

    pub fn write_byte(&mut self, chr: char) {
        let mut x = self.pos.x.get_mut();
        let mut y = self.pos.y.get_mut();
        match chr {
            '\n' => {
                *x = 0;
                *y += 16;
            }
            chr => {
                put_char(&mut self.gop, &self.font, &mut self.colour, chr, *x, *y);
                *x += 8
            }
        }
        self.check_bounds();
    }

    pub fn write_string(&mut self, s: &str) {
        for chr in s.chars() {
            self.write_byte(chr);
        }
    }

    fn check_bounds(&mut self) {
        let mut x = self.pos.x.get_mut();
        let mut y = self.pos.y.get_mut();

        // Check if next character will excede width
        let res = (self.gop.horizonal, self.gop.vertical);
        if *x + 8 > res.0 {
            *x = 0;
            *y += 16;
        }
        // Check if next line will excede height
        if *y + 16 > res.1 {
            let start_offset = (16 * 4 * res.0) as isize;
            let size = ((res.0 * res.1) - res.0) * 4;

            let buf = self.gop.buffer.get_mut();

            unsafe {
                // Copy memory from bottom to top (aka scroll)
                core::ptr::copy(buf.offset(start_offset), *buf, size);
            }
            *y -= 16;
            *x = 0
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        pos: Pos {
            x: AtomicUsize::new(0),
            y: AtomicUsize::new(0),
        },
        gop: Gop {
            buffer: AtomicPtr::default(),
            buffer_size: 0,
            horizonal: 0,
            vertical: 0,
            stride: 0,
        },
        font: super::PSF1_FONT_NULL,
        colour: AtomicU32::new(0xFF_FF_FF),
    });
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::gop::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! colour {
    ($colour:expr) => {
        $crate::gop::WRITER.lock().set_colour($colour)
    };
}

use core::fmt::Arguments;

#[doc(hidden)]
pub fn _print(args: Arguments) {
    // let mut write = &mut *;
    WRITER.lock().write_fmt(args);
}
