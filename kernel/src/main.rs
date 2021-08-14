#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;
use uefi::proto::console::gop::*;
use uefi::table::{Runtime, SystemTable};

mod gop;

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];

struct PSF1FontHeader {
    magic: [u8; 2],
    mode: u8,
    charsize: u8,
}

pub struct PSF1Font<'a> {
    psf1_header: PSF1FontHeader,
    glyph_buffer: &'a [u8],
}

// A Null psf1 font to use in place of the real PSF1 Font
pub const PSF1_FONT_NULL: PSF1Font = PSF1Font {
    psf1_header: PSF1FontHeader {
        magic: PSF1_MAGIC,
        mode: 0,
        charsize: 0,
    },
    glyph_buffer: &[0u8],
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Warning red
    colour!(0xFF_0F_0F);
    println!("{}", info);
    loop {
        // Halt processor cause why waste processor cycles
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(
    runtime_services: SystemTable<Runtime>,
    gop: gop::Gop,
    font: PSF1Font<'static>,
) -> ! {
    gop::WRITER.lock().set_gop(gop, font);

    // Print colour and scroll test
    for _ in 0..255 {
        colour!(0xFF_00_00);
        println!("Red");
        colour!(0x00_FF_00);
        println!("    Green");
        colour!(0x00_00_FF);
        println!("          Blue");
    }
    loop {
        // Halt processor cause why waste processor cycles
        unsafe {
            asm!("hlt");
        };
    }
}
