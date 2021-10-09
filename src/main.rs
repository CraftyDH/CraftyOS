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

use alloc::{string::String, vec::Vec};
use crafty_os::{
    allocator,
    disk::ata::ATA,
    driver::driver_task,
    executor::{spawner::Spawner, task::TaskPriority, yield_now, Executor},
    gdt, hlt_loop, interrupts,
    memory::{self, BootInfoFrameAllocator},
    multitasking::{self, Task, TaskManager, TASKMANAGER},
    pci::PCI,
};
use x86_64::{
    instructions::{hlt, interrupts::enable as enable_interrupts},
    VirtAddr,
};

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

//* The entry point
// Don't mangle the name so that the bootloader can run the function
// This code should never return

use bootloader::{entry_point, BootInfo};

async fn number() -> u8 {
    yield_now().await;
    32
}

async fn slow() {
    println!("Task");
    yield_now().await;
    println!("Task 2");
}

async fn example_task(spawner: Spawner) {
    let slow_id = spawner.spawn(slow(), TaskPriority::Normal);
    yield_now().await;
    spawner.kill(slow_id);

    let number = number().await;
    println!("Async number: {}", number);
}

async fn read_disks() {
    // Interrupt 14
    let mut ata_0_master = ATA::new(0x1F0, true);
    let mut ata_0_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_master_info = ata_0_master.identify(&mut ata_0_master_info).await;

    let mut ata_0_slave = ATA::new(0x1F0, false);
    let mut ata_0_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_0_slave_info = ata_0_slave.identify(&mut ata_0_slave_info).await;

    // Yeild because these instructions could be expensive
    yield_now().await;

    // Interrupt 15
    let mut ata_1_master = ATA::new(0x170, true);
    let mut ata_1_master_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_master_info = ata_1_master.identify(&mut ata_1_master_info).await;

    let mut ata_1_slave = ATA::new(0x170, false);
    let mut ata_1_slave_info: Vec<u8> = Vec::with_capacity(512);
    let ata_1_slave_info = ata_1_slave.identify(&mut ata_1_slave_info).await;

    // Yeild because these instructions could be expensive
    yield_now().await;

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

    println!("Ending")

    // let buffer = &['H' as u8, 'e' as u8, 'y' as u8, '!' as u8];
    // ata_0_master.write_28(10, buffer, 4);
    // ata_0_master.flush();

    // ata_0_master.read_28(10, 255);
}

async fn get_pci_devices(spawner: Spawner) {
    let mut pci_controller = PCI::new();
    pci_controller.select_drivers(spawner).await;
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
    let mut task_manager = TASKMANAGER.lock();

    // Spawn driver task
    let mut driver = Task::new(&mut frame_allocator, &mut mapper);
    driver.set_args_0(driver_task);
    task_manager.spawn(driver);

    // Spawn string writer
    let mut string = Task::new(&mut frame_allocator, &mut mapper);
    let (a, b, c) = String::from("Pass the value").into_raw_parts();
    string.set_args_3(task_str, a, b, c);
    task_manager.spawn(string);

    let mut chr_num = Task::new(&mut frame_allocator, &mut mapper);
    chr_num.set_args_2(task_chr_num, 'X', 99);
    task_manager.spawn(chr_num);

    // Release the mutex
    drop(task_manager);

    println!("Enabling Interrupts...");
    // Enable interrupts so that task scheduler starts
    enable_interrupts();

    println!("Waiting for task manager to take control...");

    // Wait for next tick
    hlt();

    //* This thead no longer exists
    // This is because the task manager doesn't keep any info on this thread to return
    // Therefore if this is called we have a problem
    panic!("This bootstrap thread was called");

    // The executer will run all the basic tasks which will then action drive the rest of the OS
    // println!("Initializing EXECUTOR...");
    // let mut executor = Executor::new();
    // let spawner = executor.get_spawner();

    // // Start all the interrupts first
    // spawner.spawn(keyboard::print_keypresses(), TaskPriority::Interrupt);
    // spawner.spawn(mouse::print_mousemovements(), TaskPriority::Interrupt);

    // // Then start the processes
    // spawner.spawn(read_disks(), TaskPriority::Normal);
    // spawner.spawn(get_pci_devices(spawner.clone()), TaskPriority::Normal);
    // spawner.spawn(example_task(spawner.clone()), TaskPriority::Normal);

    // println!("Starting EXECUTOR...");
    // executor.run();

    // panic!("Executor has finished :/");

    // #[cfg(test)]
    // test_main();

    // hlt_loop();
}

/// As long as this is called from rust char should be safe
extern "C" fn task_chr_num(chr: char, num: u8) {
    loop {
        unsafe { asm!("hlt") }
        println!("{}{}", chr, num);
    }
}

extern "C" fn task_str(ptr: *mut u8, length: usize, capacity: usize) {
    let _str = unsafe { Vec::from_raw_parts(ptr, length, capacity) };
    // let string = unsafe { String::from_raw_parts(chr, len, len) };
    loop {
        unsafe { asm!("hlt") };

        println!("Str {:?}", str::from_utf8(&_str).unwrap());
    }
}
