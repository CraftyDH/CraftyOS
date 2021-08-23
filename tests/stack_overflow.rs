#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate crafty_os;

use core::panic::PanicInfo;

use crafty_os::{
    gdt::{self, tss::DOUBLE_FAULT_IST_INDEX},
    qemu::{exit_qemu, QemuExitCode},
    test::panic_handler,
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Testing stack overflow...");

    gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Execution continues after stack overflow");
}

use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // For each recursion, the return address is pushed to the stack
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    panic_handler(info)
}
