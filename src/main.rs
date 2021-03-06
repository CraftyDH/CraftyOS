// Features
#![no_std] // We don't want the standard library
#![feature(asm)] // We would like to use inline assembly
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(vec_into_raw_parts)]
#![test_runner(crafty_os::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate crafty_os;
extern crate alloc;

//* Panic Handler
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use crafty_os::{
    allocator,
    disk::ata_identify,
    driver::driver_task,
    gdt, hlt_loop, interrupts,
    memory::{self, BootInfoFrameAllocator},
    multitasking::TASKMANAGER,
    pci::get_pci_devices,
    syscall::spawn_thread,
};
use x86_64::{instructions::interrupts::enable as enable_interrupts, VirtAddr};

// Panic handler for normal
// #[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crafty_os::vga_buffer::colour::{Colour, ColourCode};

    colour!(ColourCode::from_fg(Colour::LightRed));
    println!("{}", info);
    hlt_loop()
}

// // Panic handler for tests
// #[cfg(test)]
// #[panic_handler]
// fn panic(info: &PanicInfo) -> ! {
//     crafty_os::test::panic_handler(info)
// }

entry_point!(bootstrap);

fn bootstrap(boot_info: &'static BootInfo) -> ! {
    println!("Welcome to CraftyOS...\nInitalizing hardware...");

    println!("Initializing GDT...");
    gdt::init();

    println!("Initializing IDT...");
    interrupts::init_idt();

    println!("Initializing Frame Allocator...");
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(physical_memory_offset) };
    let mut frame_allocator = unsafe {
        // Init the frame allocator
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    println!("Initializing HEAP...");
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");

    println!("Initializing Task Manager...");

    TASKMANAGER.lock().init(frame_allocator, mapper);

    // Start kernel is multithreaded mode
    // Spawn driver thread
    spawn_thread(|| {
        driver_task();
    });

    // Read the PCI devices
    spawn_thread(|| get_pci_devices());

    // Perform the ATA disk check
    spawn_thread(|| ata_identify());

    // Perform A|B|C|D
    // spawn_thread(|| {
    //     let print = |char| {
    //         loop {
    //             print!("{}", char)
    //         }
    //     };

    //     spawn_thread(|| print('a'));
    //     spawn_thread(|| print('b'));
    //     spawn_thread(|| print('c'));
    //     spawn_thread(|| print('d'));
    // });

    // Enable interrupts so that task scheduler starts
    enable_interrupts();

    println!("Waiting for Task Manager to take control...");

    // Call timer to run first task
    // This function will then no longer continue to get executed ever
    unsafe { asm!("int 0x20") };

    panic!("TASKMANAGER Failed to start...");

    // #[cfg(test)]
    // test_main();

    // hlt_loop();
}
