// Features
#![no_std] // We don't want the standard library
#![feature(asm)] // We would like to use inline assembly
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate crafty_os;
extern crate alloc;

//* Panic Handler
use core::{future::Pending, panic::PanicInfo, task::Poll};

use crafty_os::{
    allocator, hlt_loop,
    memory::{self, BootInfoFrameAllocator},
    task::{executor::Executor, keyboard, Task},
    vga_buffer::colour::ColourCode,
};
use x86_64::VirtAddr;

// Panic handler for normal
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crafty_os::vga_buffer::colour::{Colour, ColourCode};

    colour!(ColourCode::from_fg(Colour::LightRed));
    println!("{}", info);
    hlt_loop()
}

// Panic handler for tests
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crafty_os::test::panic_handler(info)
}

//* The entry point
// Don't mangle the name so that the bootloader can run the function
// This code should never return

use bootloader::{entry_point, BootInfo};

async fn wait() {
    print!("|");
}

async fn number() -> u8 {
    for _ in 0..0xFF {
        wait().await;
    }
    32
}

async fn example_task() {
    let number = number().await;
    println!("Async number: {}", number);
}

entry_point!(main);

#[no_mangle]
fn main(boot_info: &'static BootInfo) -> ! {
    println!("Welcome to CraftyOS...\nInitalizing hardware...");

    crafty_os::init();

    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(physical_memory_offset) };

    let mut frame_allocator = unsafe {
        // Init the frame allocator
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");
    println!("Successfully initiated everything.");

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.spawn(Task::new(example_task()));
    executor.run();

    #[cfg(test)]
    test_main();

    hlt_loop();
}
