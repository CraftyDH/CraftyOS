use pic8259::ChainedPics;
use spin;
use x86_64::{
    instructions::{interrupts::without_interrupts, port::PortReadOnly},
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

use crate::{
    assembly::registers::Registers,
    driver::{keyboard, mouse},
    wrap_function_registers,
};

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HardwareInterruptOffset {
    Timer = PIC1_OFFSET,
    Keyboard,
    LPT1 = PIC1_OFFSET + 7,
    Mouse = PIC1_OFFSET + 12,
    ATAMaster0,
    ATASlave0,
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

pub fn set_hardware_idt(idt: &mut InterruptDescriptorTable) {
    idt[HardwareInterruptOffset::Timer.as_usize()].set_handler_fn(wrapped_timer_handler);
    idt[HardwareInterruptOffset::Keyboard.as_usize()].set_handler_fn(ps2_keyboard_handler);
    idt[HardwareInterruptOffset::LPT1.as_usize()].set_handler_fn(lpt1_probly_rubbish_handler);
    idt[HardwareInterruptOffset::Mouse.as_usize()].set_handler_fn(ps2_mouse_handler);
    idt[HardwareInterruptOffset::ATAMaster0.as_usize()].set_handler_fn(ata_master_0_handler);
    idt[HardwareInterruptOffset::ATASlave0.as_usize()].set_handler_fn(ata_slave_0_handler);
}

extern "x86-interrupt" fn lpt1_probly_rubbish_handler(_stack_frame: InterruptStackFrame) {
    println!("Recieved LPT1, (you can probably ignore this)");

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::LPT1.as_u8());
    }
}
// Wrap timer so that we can access the registers
wrap_function_registers!(timer_handler => wrapped_timer_handler);

extern "C" fn timer_handler(stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
    // print!(".");
    // We should actually do something with the timer
    // print!("{:?}", stack_frame);

    crate::multitasking::TASKMANAGER
        .try_lock()
        .unwrap()
        .switch_task_interrupt(stack_frame, regs);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Timer.as_u8());
    }
}

extern "x86-interrupt" fn ata_master_0_handler(stack_frame: InterruptStackFrame) {
    println!("ATA Master 0: {:?}", stack_frame);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::ATAMaster0.as_u8());
    }
}

extern "x86-interrupt" fn ata_slave_0_handler(stack_frame: InterruptStackFrame) {
    println!("ATA Slave 0: {:?}", stack_frame);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::ATASlave0.as_u8());
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
