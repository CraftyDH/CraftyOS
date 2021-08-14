#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate alloc;
extern crate uefi;
extern crate uefi_services;
extern crate xmas_elf;

mod gop;
mod kernel;
mod psf1;

use alloc::vec::Vec;
use core::sync::atomic::AtomicPtr;
use uefi::prelude::*;
use uefi::proto::console::gop::*;
use uefi::table::Runtime;

pub struct Gop {
    buffer: AtomicPtr<u8>,
    buffer_size: usize,
    horizonal: usize,
    vertical: usize,
    stride: usize,
}

#[entry]
fn uefi_start(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&system_table).expect_success("Failed to initialize utils");
    // reset console before doing anything else
    system_table
        .stdout()
        .reset(false)
        .expect_success("Failed to reset output buffer");

    // Log uefi version
    let rev = system_table.uefi_revision();
    info!("UEFI version {}.{}", rev.major(), rev.minor());

    // Get the boot services
    let boot_services = system_table.boot_services();

    // Get uefi basic filesystem for the partition we are on.
    let fs = boot_services
        .get_image_file_system(image_handle)
        .expect("Failed to get filesystem")
        .unwrap();
    let fs = unsafe { &mut *fs.get() };

    // Get the root aka "/"
    let mut root = fs.open_volume().unwrap().unwrap();

    // Load kernel
    let kernel_data = kernel::get_kernel(&mut root, "kernel.elf");
    let entry_point = kernel::load_kernel(boot_services, kernel_data);

    let gop = gop::initialize_gop(boot_services);
    let gopinfo = gop.current_mode_info();
    let mut gopbuf = gop.frame_buffer();
    let (horizonal, vertical) = gopinfo.resolution();
    let gopstruct = Gop {
        buffer: AtomicPtr::new(gopbuf.as_mut_ptr()),
        buffer_size: gopbuf.size(),
        horizonal,
        vertical,
        stride: gopinfo.stride(),
    };

    info!("Loaded gop");

    let font = psf1::load_psf1_font(&mut root, "zap-light16.psf");
    info!("Loaded font");

    // Get Memory Map
    let mut memory_map_buffer = vec![0u8; boot_services.memory_map_size() + 1024];
    let memory_map: Vec<uefi::table::boot::MemoryDescriptor> = boot_services
        .memory_map(&mut memory_map_buffer)
        .unwrap()
        .unwrap()
        .1
        .copied()
        .collect();

    info!("Retrieved Memory Map");
    // info!("{:?}", memory_map_buffer);

    // TODO: Why do I have to clone the table?
    let sys = unsafe { system_table.unsafe_clone() };
    // Get a buffer the size of the runtimeservices size + a buffer
    let mut mmap = vec![0u8; boot_services.memory_map_size() + 1024];
    let _runtime_table = match sys.exit_boot_services(image_handle, &mut mmap) {
        Ok(table) => table.unwrap().0,
        Err(e) => {
            error!("Error: {:?}", e);
            loop {}
        }
    };

    //* Cannot use boot services now
    let kernel_entry: fn(Gop, psf1::PSF1Font, Vec<uefi::table::boot::MemoryDescriptor>) -> ! =
        unsafe { core::mem::transmute(entry_point as *const ()) };
    //* Point of no return
    kernel_entry(gopstruct, font, memory_map);
    // Status::SUCCESS
}
