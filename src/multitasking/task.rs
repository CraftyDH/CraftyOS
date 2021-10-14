use core::{
    cell::UnsafeCell, convert::TryInto, fmt::Result, mem::transmute_copy, panic,
    sync::atomic::Ordering,
};

use alloc::{boxed::Box, sync::Arc};
use spin::Mutex;
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
    pub fn new<F>(
        frame_allocator: &mut BootInfoFrameAllocator,
        mapper: &mut OffsetPageTable<'static>,
        f: F,
    ) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
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
            run: true,
            func: Arc::new(Mutex::new(Box::new(f))),
        }
    }

    pub fn save(&mut self, stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
        self.state_isf = stack_frame.clone();
        self.state_reg = regs.clone();
    }
}
