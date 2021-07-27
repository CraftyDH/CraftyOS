#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

#[macro_use]
extern crate log;
extern crate uefi;
extern crate uefi_services;

use uefi::prelude::*;

#[entry]
fn uefi_start(image_handler: uefi::Handle, system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&system_table).expect_success("Failed to initialize utils");
    
    // reset console before doing anything else
    system_table
        .stdout()
        .reset(false)
        .expect_success("Failed to reset output buffer");

    let rev = system_table.uefi_revision();
    let (major, minor) = (rev.major(), rev.minor());

    info!("UEFI {}.{}", major, minor);
    loop {}
    Status::SUCCESS
}
