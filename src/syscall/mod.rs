use alloc::boxed::Box;
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

use crate::{assembly::registers::Registers, multitasking::TaskID, wrap_function_registers};

pub const SYSCALL_ADDR: usize = 0x80;
const ECHO: usize = 0;
const YIELD_NOW: usize = 1;
const SPAWN_THREAD: usize = 2;
const QUIT_FUNC: usize = 3;

pub fn set_syscall_idt(idt: &mut InterruptDescriptorTable) {
    idt[SYSCALL_ADDR].set_handler_fn(wrapped_syscall_handler);
}

wrap_function_registers!(syscall_handler => wrapped_syscall_handler);

extern "C" fn syscall_handler(stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
    // Run syscalls without interrupts
    // This means execution should not be interrupted
    without_interrupts(|| match regs.rax {
        ECHO => echo_handler(regs),
        YIELD_NOW => crate::multitasking::TASKMANAGER
            .lock()
            .yield_now(stack_frame, regs),
        SPAWN_THREAD => crate::multitasking::TASKMANAGER
            .lock()
            .spawn_thread_sys(regs),
        QUIT_FUNC => crate::multitasking::TASKMANAGER
            .lock()
            .quit(stack_frame, regs),
        _ => println!("Unknown syscall class: {}", regs.rax),
    })
}

unsafe fn syscall1(mut syscall_number: usize, arg1: usize) -> usize {
    asm!("int 0x80", inout("rax") syscall_number, in("r8") arg1, options(nostack));
    syscall_number
}

/// Syscall test
/// Will return number passed as arg1
pub fn echo(number: usize) -> usize {
    unsafe { syscall1(ECHO, number) }
}

fn echo_handler(regs: &mut Registers) {
    println!("Echoing: {}", regs.r8);
    regs.rax = regs.r8
}

pub fn yield_now() {
    unsafe { syscall1(YIELD_NOW, 0) };
}

pub fn spawn_thread<F>(func: F) -> TaskID
where
    F: FnOnce() + Send + Sync,
{
    let boxed_func: Box<dyn FnOnce()> = Box::new(func);
    let raw = Box::into_raw(Box::new(boxed_func)) as *mut usize;
    let res = unsafe { syscall1(SPAWN_THREAD, raw as usize) };
    TaskID::from(res)
}

pub fn quit_function() -> ! {
    unsafe { syscall1(QUIT_FUNC, 0) };

    panic!("Function failed to QUIT")
}
