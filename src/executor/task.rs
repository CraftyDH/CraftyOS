use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::Waker,
};

use alloc::{boxed::Box, sync::Arc, task::Wake};
use futures_util::Future;
use x86_64::instructions::interrupts::without_interrupts;

use super::{QueueItem, Sleep, SpawnerQueue};

pub struct Task {
    pub id: TaskID,
    pub(super) future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    pub(super) waker: Option<Arc<Waker>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            id: TaskID::new(),
            future: Box::pin(future),
            waker: None,
        }
    }

    pub(super) fn set_waker(&mut self, waker: Arc<Waker>) {
        self.waker = Some(waker)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskID(u64);

impl TaskID {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct TaskWaker {
    task_id: TaskID,
    task_queue: SpawnerQueue,
    waker: Sleep,
}

impl TaskWaker {
    pub(super) fn new(task_id: TaskID, task_queue: SpawnerQueue, waker: Sleep) -> Waker {
        Waker::from(Arc::new(Self {
            task_id,
            task_queue,
            waker,
        }))
    }
    fn wake_task(&self) {
        without_interrupts(|| {
            self.task_queue
                .lock()
                .push_back(QueueItem::Poll(self.task_id));
            self.waker.wake();
        });
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

pub enum TaskPriority {
    Interrupt,
    Normal,
}

pub(super) struct TaskQueue {
    pub interrupt: SpawnerQueue,
    pub normal: SpawnerQueue,
}

impl TaskQueue {
    pub(super) fn get(&self, priority: &TaskPriority) -> SpawnerQueue {
        match priority {
            TaskPriority::Interrupt => self.interrupt.clone(),
            TaskPriority::Normal => self.normal.clone(),
        }
        // self.high_priority.clone()
    }
}

impl Clone for TaskQueue {
    fn clone(&self) -> Self {
        TaskQueue {
            interrupt: self.interrupt.clone(),
            normal: self.normal.clone(),
        }
    }
}
