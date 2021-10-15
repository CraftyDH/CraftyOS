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
macro_rules! wrap_function_registers {
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
