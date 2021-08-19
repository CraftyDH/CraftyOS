// Features
#![no_std] // We don't want the standard library
#![no_main] // We have our own "_start" function
#![feature(asm)] // We would like to use inline assembly

//* Modules
mod vga_buffer;

#[macro_use] // Import lazy_static! macro globally
extern crate lazy_static;

// Colours
use vga_buffer::{Colour, Colour::*, ColourCode};

//* Panic Handler
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    colour!(ColourCode::from_fg(LightRed));
    println!("{}", info);
    hlt_loop()
}

//* Creates a loop and halts everytime to not waste CPU cycles
fn hlt_loop() -> ! {
    loop {
        // It should always be safe to halt
        unsafe { asm!("hlt") }
    }
}

//* The entry point
// Don't mangle the name so that the bootloader can run the function
// This code should never return
#[no_mangle]
pub extern "C" fn _start() -> ! {
    for i in 0.. {
        // Choose colour based on i mod 16 as their are 16 options
        let colour: Colour = unsafe { core::mem::transmute::<u8, Colour>((i % 16) as u8) };
        colour!(ColourCode::from_fg(colour));
        println!("Hello World! {}", i);
    }
    hlt_loop();
}
