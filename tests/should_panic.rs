#![no_std]
#![no_main]

#[macro_use]
extern crate crafty_os;

use core::panic::PanicInfo;
use crafty_os::{
    hlt_loop,
    qemu::{exit_qemu, QemuExitCode},
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
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
