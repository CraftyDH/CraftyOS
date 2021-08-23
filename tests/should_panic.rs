#![no_std]
#![no_main]

#[macro_use]
extern crate crafty_os;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use crafty_os::qemu::{exit_qemu, QemuExitCode};

entry_point!(main);

fn main(_boot_info: &'static BootInfo) -> ! {
    serial_println!("Testing panic");
    assert_eq!(0, 1); // Test something that will always be false

    serial_println!("[test did not panic]"); // it should of paniced
    exit_qemu(QemuExitCode::Failed);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
}
