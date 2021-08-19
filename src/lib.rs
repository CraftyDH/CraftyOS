#![no_std]
#![feature(asm)] // We would like to use inline assembly
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

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

use core::panic::PanicInfo;

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test::panic_handler(info)
}
