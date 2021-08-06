#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;
use uefi::proto::console::gop::*;
use uefi::table::{Runtime, SystemTable};

struct PSF1FontHeader {
    magic: [u8; 2],
    mode: u8,
    charsize: u8,
}

pub struct PSF1Font<'a> {
    psf1_header: PSF1FontHeader,
    glyph_buffer: &'a [u8],
}

struct Point {
    x: usize,
    y: usize,
}

static mut CursorPosition: Point = Point { x: 0, y: 0 };

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

fn put_char(
    fb: &mut FrameBuffer,
    fbinfo: ModeInfo,
    font: &PSF1Font,
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

fn print(fb: &mut FrameBuffer, fbinfo: ModeInfo, font: &PSF1Font, colour: u32, string: &str) {
    unsafe {
        for chr in string.chars() {
            // Check for newline
            if chr == '\n' {
                CursorPosition.x = 0;
                CursorPosition.y += 16;
            } else {
                //  Print Character
                put_char(
                    fb,
                    fbinfo,
                    font,
                    colour,
                    chr,
                    CursorPosition.x,
                    CursorPosition.y,
                );
                // Check if next character will excede width
                CursorPosition.x += 8;
                if CursorPosition.x + 8 > fbinfo.resolution().0 {
                    CursorPosition.x = 0;
                    CursorPosition.y += 16;
                }
            }
            // Check if next line will excede height
            if CursorPosition.y + 16 > fbinfo.resolution().1 {
                // Copy memory from bottom to top (aka scroll)
                core::ptr::copy(
                    fb.as_mut_ptr()
                        .offset((16 * 4 * fbinfo.resolution().0) as isize),
                    fb.as_mut_ptr(),
                    ((fbinfo.resolution().0 * fbinfo.resolution().1) - fbinfo.resolution().0) * 4,
                );
                CursorPosition.x = 0;
                CursorPosition.y -= 16;
            }
        }
    }
}

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(
    runtime_services: SystemTable<Runtime>,
    gopbuf: &mut FrameBuffer,
    gopinfo: ModeInfo,
    font: PSF1Font,
) -> ! {
    // Test print
    for _ in 0..255 {
        print(gopbuf, gopinfo, &font, 0xff_00_00, "Red\n");
        print(gopbuf, gopinfo, &font, 0x00_ff_00, "    Green\n");
        print(gopbuf, gopinfo, &font, 0x00_00_ff, "          Blue\n");
    }

    print(gopbuf, gopinfo, &font, 0xff_ff_ff, "1 + 1 = 2");

    loop {}
}
