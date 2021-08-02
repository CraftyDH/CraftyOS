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

use uefi::prelude::*;
use uefi::proto::media::file::*;

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

    // Find the kernel and open it
    let kernel = match File::open(
        &mut root,
        "kernel.elf",
        FileMode::Read,
        FileAttribute::READ_ONLY,
    ) {
        Ok(kernel) => {
            info!("Found the kernel");
            kernel.unwrap()
        }
        Err(e) => {
            info!("{:?}", e);
            loop {}
        }
    };

    // Kernal must be a file
    let mut kernel = match kernel.into_type()?.expect("Failed to get kernel") {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            info!("Kernel is a dir ???");
            loop {}
        }
    };

    // 150 Bytes for the header should be suffient
    let mut kernel_info_buffer = vec![0; 150];
    let kernel_info = match File::get_info::<FileInfo>(&mut kernel, &mut kernel_info_buffer) {
        Ok(file) => file.unwrap(),
        Err(e) if e.status() == Status::BUFFER_TOO_SMALL => {
            // Header needs a bigger buffer :(
            let size = e.data().unwrap();
            info!("Reading kernel with size {:?}", size);
            // Increase buffer to size requested
            kernel_info_buffer.resize(size, 0);
            // This time size should be right panic otherwise.
            File::get_info::<FileInfo>(&mut kernel, &mut kernel_info_buffer)
                .expect("Incorrect size given")
                .unwrap()
        }
        Err(e) => {
            info!("{:?} : {:?}", e.status(), e.data());
            loop {}
        }
    };

    // Read the kernel
    let mut kernel_data = vec![0; kernel_info.file_size() as usize];
    let bytes_read = kernel.read(&mut kernel_data).unwrap().unwrap();

    // Check that we read all of the kernel
    if bytes_read as u64 != kernel_info.file_size() {
        info!(
            "Only read {} bytes out of {}",
            bytes_read,
            kernel_info.file_size()
        )
    }

    // Use xmas elf becuase I don't want to implement ELF
    let kernel_elf = xmas_elf::ElfFile::new(&mut kernel_data).unwrap();
    // Check that it is a valid ELF file
    xmas_elf::header::sanity_check(&kernel_elf).unwrap();

    for header in kernel_elf.program_iter() {
        // Only deal with 1 type of ELF program
        if header.get_type().unwrap() == xmas_elf::program::Type::Load {
            // Round size required to pages.
            let pages = (header.mem_size() + 0x1000 - 1) / 0x1000;
            let mut segment = header.physical_addr();

            // This errors with NOT_FOUND after the first run ???, however it still works
            // TODO: Make it not error
            let _ = match boot_services.allocate_pages(
                uefi::table::boot::AllocateType::Address(segment as usize),
                uefi::table::boot::MemoryType::LOADER_DATA,
                pages as usize,
            ) {
                Err(err) => {
                    info!("{:?}", err);
                }
                Ok(_) => (),
            };

            // Get all the data from the file
            let data = match header.get_data(&kernel_elf).unwrap() {
                xmas_elf::program::SegmentData::Undefined(data) => data,
                // IDK, dont know how to handle other cases
                other => {
                    info!("ELF header returned: {:?}", other);
                    loop {}
                }
            };

            // Write each section byte by byte
            for chr in data {
                unsafe {
                    core::ptr::write(segment as *mut u8, *chr);
                }
                segment += 1;
            }
        }
    }

    // Create an unsafe pointer to the kernel entry point in memory and execute it.
    let ptr = kernel_elf.header.pt2.entry_point() as *const ();
    let kernel_entry: fn() -> u8 = unsafe { core::mem::transmute(ptr) };

    info!("Kernel said: {}", kernel_entry());

    // Infinite loop
    loop {
        unsafe { asm!("hlt") }
    }
    // Status::SUCCESS
}
