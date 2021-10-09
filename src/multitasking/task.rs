use core::{convert::TryInto, mem::transmute_copy, sync::atomic::Ordering};

use x86_64::{
    structures::{
        idt::{InterruptStackFrame, InterruptStackFrameValue},
        paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags},
    },
    VirtAddr,
};

use crate::{interrupts::hardware::Registers, memory::BootInfoFrameAllocator};

use super::{Task, TaskID, STACK_ADDR, STACK_SIZE};

impl Task {
    // C calling convention https://wiki.osdev.org/Calling_Conventions

    /// Safety for set_args_n
    /// As long as each type is not longer than 64 bits which would require 2
    pub fn set_args_3<A, B, C>(&mut self, func: extern "C" fn(A, B, C), a: A, b: B, c: C) {
        self.state_isf.instruction_pointer = VirtAddr::from_ptr(func as *const usize);
        unsafe {
            self.state_reg.rdi = transmute_copy(&a);
            self.state_reg.rsi = transmute_copy(&b);
            self.state_reg.rdx = transmute_copy(&c);
        }
    }

    /// Safety for set_args_n
    /// A, B types must not exceed a total size of 64 bits each
    pub fn set_args_2<A, B>(&mut self, func: extern "C" fn(A, B), a: A, b: B) {
        self.state_isf.instruction_pointer = VirtAddr::from_ptr(func as *const usize);
        unsafe {
            self.state_reg.rdi = transmute_copy(&a);
            self.state_reg.rsi = transmute_copy(&b);
        }
    }

    /// Safety for set_args_n
    /// A type must not exceed a total size of 64 bits each
    pub fn set_args_1<A>(&mut self, func: extern "C" fn(A), a: A) {
        self.state_isf.instruction_pointer = VirtAddr::from_ptr(func as *const usize);
        unsafe {
            self.state_reg.rdi = transmute_copy(&a);
        }
    }

    pub fn set_args_0(&mut self, func: extern "C" fn()) {
        self.state_isf.instruction_pointer = VirtAddr::from_ptr(func as *const usize);
    }

    pub fn new(frame_allocator: &mut BootInfoFrameAllocator, mapper: &mut OffsetPageTable) -> Self {
        // Allocate a new frame to store the stack in
        let frame = frame_allocator.allocate_frame().unwrap();
        let addr = STACK_ADDR.fetch_add(STACK_SIZE.try_into().unwrap(), Ordering::SeqCst);
        let page = Page::containing_address(VirtAddr::new(addr));
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        // Map the frame the virtual stack address
        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .unwrap()
                .flush();
        }

        let state_isf = InterruptStackFrameValue {
            instruction_pointer: VirtAddr::new(0),
            code_segment: 8,
            cpu_flags: 0x202,
            // cpu_flags: (RFlags::IOPL_HIGH | RFlags::IOPL_LOW | RFlags::INTERRUPT_FLAG).bits(),
            stack_pointer: VirtAddr::new(addr + STACK_SIZE as u64),
            stack_segment: 0,
        };

        Self {
            id: TaskID::new(),
            state_isf,
            state_reg: Registers::default(),
        }
    }

    pub fn save(&mut self, stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
        self.state_isf = stack_frame.clone();
        self.state_reg = regs.clone();
    }
}
