use core::intrinsics::transmute;

use pic8259::ChainedPics;
use spin;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

use crate::driver::{keyboard, mouse};

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
    idt[HardwareInterruptOffset::Timer.as_usize()].set_handler_fn(wrapped_timer_handler);
    idt[HardwareInterruptOffset::Keyboard.as_usize()].set_handler_fn(ps2_keyboard_handler);
    idt[HardwareInterruptOffset::Mouse.as_usize()].set_handler_fn(ps2_mouse_handler);
    idt[HardwareInterruptOffset::ATAMaster0.as_usize()].set_handler_fn(ata_master_0_handler);
    idt[HardwareInterruptOffset::ATASlave0.as_usize()].set_handler_fn(ata_slave_0_handler);
    idt
}

// See: https://github.com/xfoxfu/rust-xos/blob/8a07a69ef/kernel/src/interrupts/handlers.rs#L92
#[repr(align(8), C)]
#[derive(Debug, Clone, Default)]
pub struct Registers {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,
    pub rbp: usize,
}

// See: https://github.com/xfoxfu/rust-xos/blob/8a07a69ef/kernel/src/interrupts/handlers.rs#L112
/// Allows the access and modification of CPU registers
/// Args returned: (stack_frame: &mut InterruptStackFrame, regs: &mut Registers)
#[macro_export]
macro_rules! wrap {
    ($fn: ident => $w:ident) => {
        #[naked]
        pub extern "x86-interrupt" fn $w(_: InterruptStackFrame) {
            unsafe {
            asm!(
                "push rbp",
                "push rax",
                "push rbx",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push r12",
                "push r13",
                "push r14",
                "push r15",
                "mov rsi, rsp", // Arg #2: register list
                "mov rdi, rsp", // Arg #1: interupt frame
                "add rdi, 15 * 8",
                "call {}",
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rbx",
                "pop rax",
                "pop rbp",
                "iretq",
                sym $fn,
                options(noreturn)
            );
            }
        }
    };
}

// Wrap timer so that we can access the registers
wrap!(timer_handler => wrapped_timer_handler);

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
