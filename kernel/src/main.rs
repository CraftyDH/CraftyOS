#![no_std]
#![no_main]
#![feature(asm)]
use core::panic::PanicInfo;
use uefi::proto::console::gop::*;
use uefi::table::{Runtime, SystemTable};

mod bitmap;
#[macro_use]
mod gop;
mod memory;
mod paging;

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

use paging::page_frame_allocator::PageFrameAllocator;

#[no_mangle]
// Use extern win64 so params come through correctly. Thanks Microsoft
pub extern "win64" fn _start(
    mut gop: gop::Gop,
    font: PSF1Font<'static>,
    mut mmap: &mut [uefi::table::boot::MemoryDescriptor],
) -> ! {
    let mut GlobalAllocator = PageFrameAllocator::new(&mut mmap);

    let base = *gop.buffer.get_mut() as u64;
    let size = gop.buffer_size as u64;
    // Reserve GOP framebuffer
    GlobalAllocator.reserve_pages((base - 4096 * 10) as *mut u8, (size / 4096) + 2);

    // Reserve Stuff
    GlobalAllocator.reserve_pages(0 as *mut u8, 10);
    unsafe {
        // Clear screen
        core::ptr::write_bytes::<u8>(*gop.buffer.get_mut() as *mut u8, 0, gop.buffer_size);
    }
    gop::WRITER.lock().set_gop(gop, font);

    // println!("{:?}", mmap);

    // Init Paging
    unsafe {
        // Reserve Pages
        // TODO: Reserve Kernel

        use paging::page_table_manager::PageTableManager;

        let mut pml4 = GlobalAllocator.request_page() as *mut paging::PageTable;

        println!("pml4 {:?}", pml4);
        // Clear plm4
        core::ptr::write_bytes::<u8>(pml4 as *mut u8, 0, 0x1000);

        let mut page_table_manager = PageTableManager::new(pml4);

        println!("Mem {}", memory::get_memory_size(&mut mmap));

        // for t in (0..(memory::get_memory_size(&mut mmap))).step_by(0x1000) {
        for t in (0..memory::get_memory_size(&mut mmap)).step_by(0x1000) {
            page_table_manager.map_memory(&mut GlobalAllocator, t as *const u8, t as *const u8)
        }

        println!("T");

        for t in (base..(base + size)).step_by(0x1000) {
            page_table_manager.map_memory(&mut GlobalAllocator, t as *const u8, t as *const u8)
        }

        // Activate new page map
        // TODO: Make it not crash
        asm!("mov cr3, {}", inout(reg) pml4);

        // * Test new page map
        println!("Didn't crash");

        page_table_manager.map_memory(
            &mut GlobalAllocator,
            0x6_000_000 as *const u8,
            0x5_000_000 as *const u8,
        );
        core::ptr::write(0x5_000_000 as *mut u8, 27);
        println!("Number {}", core::ptr::read(0x6_000_000 as *mut u8));
    }

    loop {
        // Halt processor cause why waste processor cycles
        unsafe {
            asm!("hlt");
        };
    }
}
