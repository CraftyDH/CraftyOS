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
use core::{panic::PanicInfo, str};

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use crafty_os::{
    allocator,
    disk::ata::ATA,
    driver::driver_task,
    gdt, hlt_loop, interrupts,
    memory::{self, BootInfoFrameAllocator},
    multitasking::TASKMANAGER,
    pci::PCI,
    syscall::{spawn_thread, yield_now},
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

fn read_disks() {
    // Interrupt 14
    let mut ata_0_master = ATA::new(0x1F0, true);
    let mut ata_0_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_master_info = ata_0_master.identify(&mut ata_0_master_info);

    let mut ata_0_slave = ATA::new(0x1F0, false);
    let mut ata_0_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_slave_info = ata_0_slave.identify(&mut ata_0_slave_info);

    // Interrupt 15
    let mut ata_1_master = ATA::new(0x170, true);
    let mut ata_1_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_master_info = ata_1_master.identify(&mut ata_1_master_info);

    let mut ata_1_slave = ATA::new(0x170, false);
    let mut ata_1_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_slave_info = ata_1_slave.identify(&mut ata_1_slave_info);

    for (ata_info, name) in [
        (ata_0_master_info, "ATA 0 Master"),
        (ata_0_slave_info, "ATA 0 Slave"),
        (ata_1_master_info, "ATA 1 Master"),
        (ata_1_slave_info, "ATA 1 Slave"),
    ] {
        if let Some(info) = ata_info {
            println!("Found drive on {}", name);
            println!(
                "    Serial: {}\n    Model:  {}",
                str::from_utf8(&info.serial).unwrap_or("INVALID SERIAL"),
                str::from_utf8(&info.model).unwrap_or("INVALID MODEL"),
            );
        } else {
            println!("No drive found on {}", name)
        }
    }

    // let buffer = &['H' as u8, 'e' as u8, 'y' as u8, '!' as u8];
    // ata_0_master.write_28(10, buffer, 4);
    // ata_0_master.flush();

    // ata_0_master.read_28(10, 255);
}

fn get_pci_devices() {
    let mut pci_controller = PCI::new();
    pci_controller.select_drivers();
}

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

    // Read disks
    spawn_thread(|| {
        get_pci_devices();
        read_disks();
    });

    spawn_thread(|| {
        let mut x = 5;

        spawn_thread(|| {
            for _ in 0..0xFFFF {
                yield_now();
                yield_now();
                yield_now();
                yield_now();
                yield_now();
            }
            x = 555;
        });
        while x == 5 {
            yield_now()
        }
        println!("X: {}", x)
    });

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
