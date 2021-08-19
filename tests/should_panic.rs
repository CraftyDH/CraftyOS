#![no_std]
#![no_main]

#[macro_use]
extern crate crafty_os;

use core::panic::PanicInfo;
use crafty_os::qemu::{exit_qemu, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    panic!("Testing panic");

    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
