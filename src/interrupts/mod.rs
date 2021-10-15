use x86_64::structures::idt::InterruptDescriptorTable;

pub mod exceptions;
pub mod hardware;

use lazy_static::lazy_static;

use crate::syscall;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Set idt table
        exceptions::set_exceptions_idt(&mut idt);
        hardware::set_hardware_idt(&mut idt);
        syscall::set_syscall_idt(&mut idt);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
    unsafe { hardware::PICS.lock().initialize() };
}
