#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;
use uefi::proto::console::gop::*;
use uefi::table::{Runtime, SystemTable};

mod gop;

struct PSF1FontHeader {
    magic: [u8; 2],
    mode: u8,
    charsize: u8,
}

pub struct PSF1Font<'a> {
    psf1_header: PSF1FontHeader,
    glyph_buffer: &'a [u8],
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(
    runtime_services: SystemTable<Runtime>,
    gopbuf: &'static mut FrameBuffer<'static>,
    gopinfo: ModeInfo,
    font: PSF1Font<'static>,
) -> ! {
    let gop = gop::Gop {
        buffer: gopbuf,
        info: gopinfo,
        font,
    };

    gop::WRITER.lock().set_gop(gop);

    // println!("Hello world{}", "!");

    println!("Hello");

    colour!(0xFF_00_00);

    println!("Hello");

    // // Test print
    for _ in 0..255 {
        colour!(0xFF_00_00);
        println!("Red");
        colour!(0x00_FF_00);
        println!("    Green");
        colour!(0x00_00_FF);
        println!("          Blue");
    }

    loop {}
}
