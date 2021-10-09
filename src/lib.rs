#![no_std]
#![feature(asm)] // We would like to use inline assembly
#![feature(llvm_asm)]
#![feature(abi_x86_interrupt)] // So we can handle iterrupts with the abi
#![feature(alloc_error_handler)] // We need to be able to create the error handler
#![feature(const_mut_refs)] // So mutable refrences can be in a const function
#![feature(generators)]
#![feature(naked_functions)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(linked_list_cursors)]
#![feature(core_intrinsics)]

#[macro_use] // Import lazy_static! macro globally
extern crate lazy_static;

// So we can implement our heap allocator :)
extern crate alloc;

//* Modules
pub mod qemu;
#[macro_use]
pub mod serial;
pub mod test;
#[macro_use]
pub mod vga_buffer;
pub mod allocator;
pub mod disk;
pub mod driver;
pub mod executor;
pub mod gdt;
pub mod interrupts;
pub mod locked_mutex;
pub mod memory;
pub mod multitasking;
pub mod pci;

#[cfg(test)]
use core::panic::PanicInfo;

use x86_64::instructions::hlt;

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test::panic_handler(info)
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation Error: {:?}", layout)
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
    // x86_64::instructions::interrupts::enable();
}

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop()
}
