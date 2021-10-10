use core::ptr::write_volatile;

use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use spin::Mutex;
use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue};

use crate::interrupts::hardware::Registers;

use super::{Task, TaskID, TaskManager};

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            current_task: TaskID::none_task(),
        }
    }

    pub fn spawn(&mut self, task: Task) -> bool {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            println!("Task with same ID already exists in tasks");
            return false;
        }
        self.task_queue.lock().push_back(task_id);
        true
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
            let next_task = self.tasks.get(&next_task_id).unwrap();
            self.current_task = next_task_id;

            unsafe {
                let stack_frame_mut = stack_frame.as_mut();

                // TODO: Fix writing
                // It is volitile to avoid optimisations
                // However I must use "extract_inner"
                // stack_frame_mut.write(next_task.state_isf.clone());

                let inner = stack_frame_mut.extract_inner();

                // Write new stack_frame of new process
                write_volatile(
                    inner as *mut InterruptStackFrameValue,
                    next_task.state_isf.clone(),
                );

                // let sf = stack_frame_mut.extract_inner();
                // sf.instruction_pointer = next_task.state_isf.instruction_pointer;
                // sf.stack_pointer = next_task.state_isf.stack_pointer;
                // // sf.stack_segment = next_task.state_isf.stack_segment;
                // sf.cpu_flags = next_task.state_isf.cpu_flags;

                // Write cpu registers to what the new process expects
                *regs = next_task.state_reg.clone();
            }
        }
        // Otherwise continue running the only task
    }
}
