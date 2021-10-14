pub mod task;
pub mod taskmanager;

use core::sync::atomic::{AtomicU64, Ordering};

use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use spin::Mutex;
use x86_64::structures::{idt::InterruptStackFrameValue, paging::OffsetPageTable};

use crate::{interrupts::hardware::Registers, memory::BootInfoFrameAllocator};

// Start stack at this address
static STACK_ADDR: AtomicU64 = AtomicU64::new(0x80_000);
const STACK_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskID(u64);

impl TaskID {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    // We use TaskID of 0 to indicate a non task
    pub fn is_none(&self) -> bool {
        if self.0 == 0 {
            return true;
        }
        false
    }

    pub fn none_task() -> Self {
        Self(0)
    }
}

type Func = Arc<Mutex<dyn Fn() + Send + Sync>>;
pub struct Task {
    pub id: TaskID,
    state_isf: InterruptStackFrameValue,
    state_reg: Registers,
    run: bool,
    func: Func,
}

lazy_static! {
    pub static ref TASKMANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

// pub const TASKMANAGER: OnceCell<Mutex<TaskManager>> = OnceCell::uninit();

struct TaskManagerInit {
    frame_allocator: BootInfoFrameAllocator,
    mapper: OffsetPageTable<'static>,
}
pub struct TaskManager {
    tasks: BTreeMap<TaskID, Task>,
    task_queue: Arc<Mutex<VecDeque<TaskID>>>,
    current_task: TaskID,
    dynamic: Option<TaskManagerInit>,
}
