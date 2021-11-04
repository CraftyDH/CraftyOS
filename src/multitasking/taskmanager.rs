use core::ptr::write_volatile;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use spin::Mutex;
use x86_64::{VirtAddr, instructions::{hlt, interrupts::enable_and_hlt}, software_interrupt, structures::{idt::{InterruptStackFrame, InterruptStackFrameValue}, paging::OffsetPageTable}};

use crate::{
    assembly::registers::Registers, executor::task, memory::BootInfoFrameAllocator,
    syscall::quit_function,
};

use super::{Task, TaskID, TaskManager, TaskManagerInit, TASKMANAGER};

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            current_task: TaskID::none_task(),
            dynamic: None,
        }
    }

    pub fn init(
        &mut self,
        mut frame_allocator: BootInfoFrameAllocator,
        mut mapper: OffsetPageTable<'static>,
    ) {
        // Create a nop task which hlt's every time
        let mut nop_task = Task::new(&mut frame_allocator, &mut mapper);
        nop_task.id = TaskID::none_task();
        nop_task.state_isf.instruction_pointer = VirtAddr::from_ptr(nop_function as *const usize);

        self.tasks.insert(TaskID::none_task(), nop_task);

        self.dynamic = Some(TaskManagerInit {
            frame_allocator,
            mapper,
        });
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            println!("Task with same ID already exists in tasks");
        }
        self.task_queue.lock().push_back(task_id);
    }

    /// To be called from syscall
    pub fn spawn_thread_sys(&mut self, regs: &mut Registers) {
        let mut task_queue = self.task_queue.lock();

        if let Some(dynamic) = &mut self.dynamic {
            let mut task = Task::new(&mut dynamic.frame_allocator, &mut dynamic.mapper);
            let task_id = task.id;

            // Return task id as successfull result
            regs.rax = task_id.0 as usize;

            // Set startpoint to bootstraper
            task.state_isf.instruction_pointer = *THREAD_BOOTSTRAPER;

            // Pass function to first param
            task.state_reg.rdi = regs.r8;

            if self.tasks.insert(task.id, task).is_some() {
                println!("Task with same ID already exists in tasks");
            }
            task_queue.push_back(task_id);
        } else {
            println!("TaskManager not initialized, dropping new thread");
        }
    }

    // pub fn run_new_func(&mut self) -> Func {
    //     println!("Spawning new thread");
    //     let task = self.tasks.get(&self.current_task).unwrap();

    //     task.func.clone()
    // }

    pub fn quit(&mut self, stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
        self.tasks.remove(&self.current_task);
        self.current_task = TaskID::none_task();

        // Switch to next task
        self.switch_task_interrupt(stack_frame, regs)
    }

    unsafe fn set_registers(
        &mut self,
        stack_frame: &mut InterruptStackFrame,
        regs: &mut Registers,
        task_id: TaskID,
    ) {
        // Get the new task's task data
        let task = self.tasks.get_mut(&task_id).unwrap();

        // Write the new tasks stack frame

        // TODO: Make this work again
        // stack_frame.as_mut().write(task.state_isf);
        // Bad solution
        write_volatile(stack_frame.as_mut().extract_inner() as *mut InterruptStackFrameValue, task.state_isf.clone());

        // Write the new tasks CPU registers
        write_volatile(regs, task.state_reg.clone());
    }

    pub fn yield_now(&mut self, stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
        let mut task_queue = self.task_queue.lock();
        task_queue.push_back(self.current_task);

        if let Some(mut next_task) = task_queue.pop_front() {
            // Save current task
            self.tasks
                .get_mut(&self.current_task)
                .unwrap()
                .save(stack_frame, regs);

            // Since they yielded if we get the task again
            // Execute none task
            if self.current_task == next_task {
                next_task = TaskID::none_task();
                // Try again next tick
                task_queue.push_back(self.current_task);
            }

            self.current_task = next_task;

            drop(task_queue);

            unsafe { self.set_registers(stack_frame, regs, next_task) }
        }
    }

    pub fn switch_task_interrupt(
        &mut self,
        stack_frame: &mut InterruptStackFrame,
        regs: &mut Registers,
    ) {
        let mut task_queue = self.task_queue.lock();

        // If task is none don't save
        if !self.current_task.is_none() {
            self.tasks
                .get_mut(&self.current_task)
                .unwrap()
                .save(stack_frame, regs);

            // Push the current task to the back of the queue
            task_queue.push_back(self.current_task);
        }

        // Can we get a new task from the queue
        if let Some(next_task_id) = task_queue.pop_front() {
            // If we got the same task as before keep running it
            if self.current_task == next_task_id {
                return;
            }

            // Set current task to our new task
            self.current_task = next_task_id;

            drop(task_queue);

            unsafe { self.set_registers(stack_frame, regs, next_task_id) };
        }
    }
}

// pub fn spawn_thread<F>(func: F)
// where
//     F: Fn() + Send + Sync + 'static,
// {
//     TASKMANAGER.lock().spawn_thread(func)
// }

lazy_static! {
    static ref THREAD_BOOTSTRAPER: VirtAddr =
        VirtAddr::from_ptr(thread_bootstraper as *const usize);
}

/// Gets executed when there are no tasks ready
/// Waits until next timer for another task to take over
extern "C" fn nop_function() -> ! {
    loop {
        enable_and_hlt()
    }
}

extern "C" fn thread_bootstraper(main: *mut usize) {
    // Recreate the function box that was passed from the syscall
    let func = unsafe { Box::from_raw(main as *mut Box<dyn FnOnce()>) };

    // Call the function
    func.call_once(());

    // Function ended quit
    quit_function()
}
