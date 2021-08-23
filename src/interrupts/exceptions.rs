use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::gdt::tss;

pub fn set_exceptions_idt(idt: &mut InterruptDescriptorTable) -> &mut InterruptDescriptorTable {
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(tss::DOUBLE_FAULT_IST_INDEX);
    }

    idt
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    // test a break point
    // Execution should continue therefore we can test this here.
    x86_64::instructions::interrupts::int3();
}
