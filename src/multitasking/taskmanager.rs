use core::{borrow::Borrow, cell::RefCell, ptr::write_volatile, sync::atomic::AtomicPtr};

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use spin::Mutex;
use x86_64::{
    instructions::interrupts::without_interrupts,
    software_interrupt,
    structures::{
        idt::{InterruptStackFrame, InterruptStackFrameValue},
        paging::OffsetPageTable,
    },
    VirtAddr,
};

use crate::{interrupts::hardware::Registers, memory::BootInfoFrameAllocator};

use super::{Func, Task, TaskID, TaskManager, TaskManagerInit, TASKMANAGER};

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
        frame_allocator: BootInfoFrameAllocator,
        mapper: OffsetPageTable<'static>,
    ) {
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

    pub fn spawn_thread<F>(&mut self, func: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Some(dynamic) = &mut self.dynamic {
            let task = Task::new(&mut dynamic.frame_allocator, &mut dynamic.mapper, func);
            self.spawn(task);
        } else {
            println!("TaskManager not initialized, dropping new thread");
        }
    }

    pub fn run_new_func(&mut self) -> Func {
        println!("Spawning new thread");
        let task = self.tasks.get(&self.current_task).unwrap();

        task.func.clone()
    }

    pub fn quit(&mut self) {
        self.tasks.remove(&self.current_task);
        self.current_task = TaskID::none_task();
    }

    pub fn switch_task_interrupt(
        &mut self,
        stack_frame: &mut InterruptStackFrame,
        regs: &mut Registers,
    ) {
        let mut task_queue = self.task_queue.lock();
        if self.tasks.is_empty() {
            return;
        }

        // If first task don't save
        if !self.current_task.is_none() {
            // println!("Saving\n\n");
            // Save current task
            self.tasks
                .get_mut(&self.current_task)
                .unwrap()
                .save(stack_frame, regs);

            // Push the old task to the back of the queue
            task_queue.push_back(self.current_task);
        } else {
            println!("Starting first task...\n");
        }

        // println!("Switching process {:?}", task_queue);

        // Can we get another task
        if let Some(next_task_id) = task_queue.pop_front() {
            // println!("Next: {:?}", next_task_id);
            let mut next_task = self.tasks.get_mut(&next_task_id).unwrap();
            self.current_task = next_task_id;

            unsafe {
                let stack_frame_mut = stack_frame.as_mut();

                // TODO: Fix writing
                // It is volitile to avoid optimisations
                // However I must use "extract_inner"
                // stack_frame_mut.write(next_task.state_isf.clone());

                let inner = stack_frame_mut.extract_inner();

                // If first run, run the thread bootstraper
                if next_task.run {
                    next_task.run = false;
                    inner.instruction_pointer = *THREAD_BOOTSTRAPER;
                } else {
                    inner.instruction_pointer = next_task.state_isf.instruction_pointer;
                }

                inner.stack_pointer = next_task.state_isf.stack_pointer;
                inner.cpu_flags = next_task.state_isf.cpu_flags;

                *regs = next_task.state_reg.clone();

                // Write new stack_frame of new process
                // write_volatile(
                //     inner as *mut InterruptStackFrameValue,
                //     next_task.state_isf.clone(),
                // );

                // let sf = stack_frame_mut.extract_inner();
                // sf.instruction_pointer = next_task.state_isf.instruction_pointer;
                // // sf.stack_segment = next_task.state_isf.stack_segment;

                // Write cpu registers to what the new process expects
            }
        }
        // Otherwise continue running the only task
    }
}

pub fn spawn_thread<F>(func: F)
where
    F: Fn() + Send + Sync + 'static,
{
    TASKMANAGER.lock().spawn_thread(func)
}

lazy_static! {
    static ref THREAD_BOOTSTRAPER: VirtAddr =
        VirtAddr::from_ptr(thread_bootstraper as *const usize);
}
extern "C" fn thread_bootstraper() {
    // Get function refrence
    let function = super::TASKMANAGER.lock().run_new_func();

    // Call the function
    function.as_ref().lock().call(());
    // Function ended quit
    super::TASKMANAGER.lock().quit();

    unsafe { software_interrupt!(0x20) }
    // Do nothing
    loop {}
}
