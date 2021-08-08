use core::fmt::Write;
use uefi::proto::console::gop::*;

fn put_char(
    fb: &mut FrameBuffer,
    fbinfo: ModeInfo,
    font: &super::PSF1Font,
    colour: u32,
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
                let loc = ((x as usize) + (y as usize * fbinfo.stride())) * 4;
                unsafe { fb.write_value(loc, colour) }
            }
        }
        addr += 1;
    }
}

pub struct Pos {
    pub x: usize,
    pub y: usize,
}
pub struct Gop {
    pub buffer: &'static mut FrameBuffer<'static>,
    pub info: ModeInfo,
    pub font: super::PSF1Font<'static>,
}

pub struct Writer {
    pos: Pos,
    gop: Option<Gop>,
    colour: u32,
}

unsafe impl Send for Writer {}
unsafe impl Send for Gop {}

impl Writer {
    pub fn set_gop(&mut self, gop: Gop) {
        self.gop = Some(gop);
    }
    pub fn set_colour(&mut self, colour: u32) -> &mut Writer {
        self.colour = colour;
        return self;
    }

    pub fn write_byte(&mut self, chr: char) {
        let gop = match &mut self.gop {
            Some(gop) => gop,
            None => return,
        };
        match chr {
            '\n' => {
                self.pos.x = 0;
                self.pos.y += 16;
            }
            chr => {
                put_char(
                    gop.buffer,
                    gop.info,
                    &gop.font,
                    self.colour,
                    chr,
                    self.pos.x,
                    self.pos.y,
                );
                self.pos.x += 8
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
        let gop = match &mut self.gop {
            Some(gop) => gop,
            None => return,
        };
        // Check if next character will excede width
        let res = gop.info.resolution();
        if self.pos.x + 8 > res.0 {
            self.pos.x = 0;
            self.pos.y += 16;
        }
        // Check if next line will excede height
        if self.pos.y + 16 > res.1 {
            let start_offset = (16 * 4 * res.0) as isize;
            let size = ((res.0 * res.1) - res.0) * 4;
            unsafe {
                // Copy memory from bottom to top (aka scroll)
                core::ptr::copy(
                    gop.buffer.as_mut_ptr().offset(start_offset),
                    gop.buffer.as_mut_ptr(),
                    size,
                );
            }
            self.pos.y -= 16;
            self.pos.x = 0
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        pos: Pos { x: 0, y: 0 },
        gop: None,
        colour: 0xFF_FF_FF
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
    WRITER.lock().write_fmt(args);
}
