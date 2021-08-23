#![no_std]
#![feature(asm)] // We would like to use inline assembly
#![feature(abi_x86_interrupt)] // So we can handle iterrupts with the abi
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use] // Import lazy_static! macro globally
extern crate lazy_static;

//* Modules
pub mod qemu;
#[macro_use]
pub mod serial;
pub mod test;
#[macro_use]
pub mod vga_buffer;
pub mod gdt;
pub mod interrupts;

#[cfg(test)]
use core::panic::PanicInfo;

use x86_64::instructions::hlt;

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test::panic_handler(info)
}

//* Creates a loop and halts everytime to not waste CPU cycles
pub fn hlt_loop() -> ! {
    loop {
        hlt()
    }
}

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    x86_64::instructions::interrupts::enable();
}

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    loop {}
}
