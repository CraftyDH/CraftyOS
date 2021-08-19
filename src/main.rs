// Features
#![no_std] // We don't want the standard library
#![feature(asm)] // We would like to use inline assembly
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate crafty_os;

//* Panic Handler
use core::panic::PanicInfo;

// Panic handler for normal
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop()
}

// Panic handler for tests
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crafty_os::test::panic_handler(info)
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
    println!("Hello World!");

    #[cfg(test)]
    test_main();

    hlt_loop();
}
