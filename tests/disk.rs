#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use crafty_os::{
    allocator::{self, HEAP_SIZE},
    disk::ata::ATA,
    hlt_loop,
    memory::{self, BootInfoFrameAllocator},
};
use x86_64::VirtAddr;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    crafty_os::init();

    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(physical_memory_offset) };

    let mut frame_allocator = unsafe {
        // Init the frame allocator
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");

    let mut ata_0_master = ATA::new(0x1F0, true);

    test_main();
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crafty_os::test::panic_handler(info)
}

#[test_case]
fn simple_allocation() {
    // Check we can allocate values and read them correctly
    let heap_value_1 = Box::new(42);
    let heap_value_2 = Box::new(123);
    assert_eq!(*heap_value_1, 42);
    assert_eq!(*heap_value_2, 123);
}
