#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use crafty_os::println;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
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
