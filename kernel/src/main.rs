#![no_std]
#![no_main]
#![feature(ptr_internals)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate bitflags;

use core::panic::PanicInfo;

mod bitmap;
#[macro_use]
mod gop;
mod gdt;
mod interrupts;

// mod paging;

// extern crate alloc;

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

// extern "C" {
//     static _KernelStart: u64;
//     static _KernelEnd: u64;
// }

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(
    mut gop: gop::Gop,
    font: PSF1Font<'static>,
    mut mmap: &mut [uefi::table::boot::MemoryDescriptor],
) -> ! {
    // let gop_entry = *gop.buffer.get_mut() as usize;
    // let gop_size = gop.buffer_size;
    gop::WRITER.lock().set_gop(gop, font);
    println!("Hello World!");

    // unsafe {
    //     println!("From {} to {}", &_KernelStart, &_KernelEnd);
    // }
    println!(
        "Interrupts: {}",
        x86_64::instructions::interrupts::are_enabled()
    );

    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() }
    x86_64::instructions::interrupts::enable();

    println!(
        "Interrupts: {}",
        x86_64::instructions::interrupts::are_enabled()
    );

    println!("End of execution. \nCraftyOS will now goto sleep...");

    loop {
        // print!("|");
        // Halt processor cause why waste processor cycles
        // unsafe {
        //     asm!("hlt");
        // };
    }
}
