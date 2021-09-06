use core::{
    convert::TryInto,
    sync::atomic::{AtomicI16, AtomicUsize},
};

use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
use pic8259::ChainedPics;
use ps2_mouse::{Mouse, MouseState};
use spin::{self, Mutex};
use x86_64::{
    instructions::port::{Port, PortReadOnly},
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

use crate::vga_buffer::{BUFFER_HEIGHT, BUFFER_WIDTH};

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HardwareInterruptOffset {
    Timer = PIC1_OFFSET,
    Keyboard,
    Mouse = PIC1_OFFSET + 12,
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

    MOUSE.lock().init().unwrap();
    MOUSE.lock().set_on_complete(mouse_packet_handler);

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
    static ref MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());
}

extern "x86-interrupt" fn ps2_keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = PortReadOnly::new(0x60);

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

extern "x86-interrupt" fn ps2_mouse_handler(_stack_frame: InterruptStackFrame) {
    let mut mouse = MOUSE.lock();
    let mut port = PortReadOnly::new(0x60);

    let packet: u8 = unsafe { port.read() };

    mouse.process_packet(packet);

    // Tell the PICS that we have handled the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(HardwareInterruptOffset::Mouse.as_u8());
    }
}

static mut X: AtomicUsize = AtomicUsize::new(0);
static mut Y: AtomicUsize = AtomicUsize::new(0);

fn mouse_packet_handler(mouse_state: MouseState) {
    let x = unsafe { X.get_mut() };
    let y = unsafe { Y.get_mut() };
    if mouse_state.x_moved() {
        *x = match (*x as isize).checked_add(mouse_state.get_x().try_into().unwrap()) {
            Some(x) => {
                let x: usize = x.try_into().unwrap_or(0);
                if x > BUFFER_WIDTH - 1 {
                    BUFFER_WIDTH - 1
                } else {
                    x
                }
            }
            None => 0,
        };
    }

    if mouse_state.y_moved() {
        *y = match (*y as isize).checked_add((-mouse_state.get_y()).try_into().unwrap()) {
            Some(y) => {
                let y: usize = y.try_into().unwrap_or(0);
                if y > BUFFER_HEIGHT - 1 {
                    BUFFER_HEIGHT - 1
                } else {
                    y
                }
            }
            None => 0,
        };
    }

    let pos = *y * BUFFER_WIDTH + *x;

    let mut a = Port::<u8>::new(0x3D4);
    let mut b = Port::<u8>::new(0x3D5);
    unsafe {
        a.write(0x0F);
        b.write((pos & 0xFF).try_into().unwrap());
        a.write(0x0E);
        b.write((pos >> 8 & 0xFF).try_into().unwrap());
    }
}
