use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
use pic8259::ChainedPics;
use spin::{self, Mutex};
use x86_64::{
    instructions::port::Port,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HardwareInterruptOffset {
    Timer = PIC1_OFFSET,
    Keyboard,
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
    idt[HardwareInterruptOffset::Keyboard.as_usize()].set_handler_fn(keyboard_handler);

    idt
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    // We should actually do something with the timer
    print!(".");

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Timer.as_u8());
    }
}

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            layouts::Us104Key,
            ScancodeSet1,
            pc_keyboard::HandleControl::Ignore
        ));
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Keyboard.as_u8());
    }
}
