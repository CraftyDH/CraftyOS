#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use crafty_os::{hlt_loop, println};

use bootloader::{entry_point, BootInfo};

entry_point!(main);

fn main(_boot_info: &'static BootInfo) -> ! {
    test_main();

    hlt_loop()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crafty_os::test::panic_handler(info)
}

// Test we can print
#[test_case]
fn test_println() {
    println!("test_println output");
}
