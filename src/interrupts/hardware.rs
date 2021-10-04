use pic8259::ChainedPics;
use spin;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
    VirtAddr,
};

use crate::task::{keyboard, mouse};

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HardwareInterruptOffset {
    Timer = PIC1_OFFSET,
    Keyboard,
    Mouse = PIC1_OFFSET + 12,
    ATAMaster0 = PIC1_OFFSET + 14,
    ATASlave0 = PIC1_OFFSET + 15,
}

impl HardwareInterruptOffset {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC1_OFFSET, PIC2_OFFSET) });

pub fn set_hardware_idt(idt: &mut InterruptDescriptorTable) -> &mut InterruptDescriptorTable {
    idt[HardwareInterruptOffset::Timer.as_usize()].set_handler_fn(timer_handler);
    idt[HardwareInterruptOffset::Keyboard.as_usize()].set_handler_fn(ps2_keyboard_handler);
    idt[HardwareInterruptOffset::Mouse.as_usize()].set_handler_fn(ps2_mouse_handler);
    idt[HardwareInterruptOffset::ATAMaster0.as_usize()].set_handler_fn(ata_handler);
    idt[HardwareInterruptOffset::ATASlave0.as_usize()].set_handler_fn(ata_handler);
    idt
}

extern "x86-interrupt" fn ata_handler(stack_frame: InterruptStackFrame) {
    println!("ATA Interrupt: {:?}", stack_frame);
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    // We should actually do something with the timer
    // print!(".");

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Timer.as_u8());
    }
}

extern "x86-interrupt" fn ps2_keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut port = PortReadOnly::new(0x60);

    let scancode: u8 = unsafe { port.read() };

    keyboard::add_scancode(scancode);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn ps2_mouse_handler(_stack_frame: InterruptStackFrame) {
    let mut port = PortReadOnly::new(0x60);

    let packet: u8 = unsafe { port.read() };

    mouse::add_scancode(packet);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Mouse.as_u8());
    }
}
