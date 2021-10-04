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
use core::{
    cell::{Ref, RefCell},
    future::Pending,
    panic::PanicInfo,
    str,
    task::Poll,
};

use alloc::{
    rc::Rc,
    string::String,
    vec::{self, Vec},
};
use crafty_os::{
    allocator,
    disk::ata::{ATADiskIdentify, ATA},
    hlt_loop,
    memory::{self, BootInfoFrameAllocator},
    pci::PCI,
    task::{executor::Executor, keyboard, mouse, yield_now, Task},
    vga_buffer::{colour::ColourCode, writer::WRITER},
};
use spin::Mutex;
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

async fn number() -> u8 {
    32
}

async fn example_task() {
    let number = number().await;
    println!("Async number: {}", number);
}

async fn read_disks() {
    println!("Yielding");
    // yield_now().await;
    // Interrupt 14
    let mut ata_0_master = ATA::new(0x1F0, true);
    let mut ata_0_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_master_info = ata_0_master.identify(&mut ata_0_master_info).await;

    let mut ata_0_slave = ATA::new(0x1F0, false);
    let mut ata_0_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_slave_info = ata_0_slave.identify(&mut ata_0_slave_info).await;

    println!("Yielding");
    // yield_now().await;

    // Interrupt 15
    // let mut ata_1_master = ATA::new(0x170, true);
    // let mut ata_1_master_info: Vec<u8> = Vec::with_capacity(512);
    // let ata_1_master_info = ata_1_master.identify(&mut ata_1_master_info).await;

    // let mut ata_1_slave = ATA::new(0x170, false);
    // let mut ata_1_slave_info: Vec<u8> = Vec::with_capacity(512);
    // let ata_1_slave_info = ata_1_slave.identify(&mut ata_1_slave_info).await;

    println!("Yielding");
    // yield_now().await;

    for (ata_info, name) in [
        (ata_0_master_info, "ATA 0 Master"),
        (ata_0_slave_info, "ATA 0 Slave"),
        // (ata_1_master_info, "ATA 1 Master"),
        // (ata_1_slave_info, "ATA 1 Slave"),
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

    println!("Ending")

    // let buffer = &['H' as u8, 'e' as u8, 'y' as u8, '!' as u8];
    // ata_0_master.write_28(10, buffer, 4);
    // ata_0_master.flush();

    // ata_0_master.read_28(10, 255);
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

    // let mut pci_controller = PCI::new();
    // pci_controller.select_drivers();

    // The executer will run all the basic tasks which will then action drive the rest of the OS
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    // executor.spawn(Task::new(mouse::print_mousemovements()));
    executor.spawn(Task::new(read_disks()));
    executor.spawn(Task::new(example_task()));
    executor.run();

    #[cfg(test)]
    test_main();

    hlt_loop();
}
