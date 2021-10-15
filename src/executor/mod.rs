use alloc::{collections::VecDeque, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use spin::Mutex;

use crate::syscall::yield_now;

use self::{
    spawner::Spawner,
    task::{Task, TaskID, TaskPriority, TaskQueue, TaskWaker},
};

pub mod spawner;
pub mod task;

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::collections::BTreeMap;

type SpawnerQueue = Arc<Mutex<VecDeque<QueueItem>>>;

#[derive(Default, Clone, Debug)]
pub struct Sleep(Arc<AtomicBool>);

impl Sleep {
    pub fn sleep(&self) {
        loop {
            if self.0.swap(false, Ordering::Acquire) {
                break;
            } else {
                // hlt();
                // Create a timer interrupt
                yield_now()
            }
        }
    }

    pub fn wake(&self) {
        self.0.store(true, Ordering::Release)
    }
}

pub struct Executor {
    tasks: BTreeMap<TaskID, Task>,
    task_queue: TaskQueue,
    sleep_waker: Sleep,
}

impl Executor {
    pub fn new() -> Self {
        let task_queue = TaskQueue {
            interrupt: Arc::new(Mutex::new(VecDeque::with_capacity(25))),
            normal: Arc::new(Mutex::new(VecDeque::with_capacity(50))),
        };

        Self {
            tasks: BTreeMap::new(),
            task_queue: task_queue,
            sleep_waker: Sleep::default(),
        }
    }

    fn spawn(&mut self, task: Task, priority: TaskPriority) {
        let task_id = task.id;
        if self.tasks.insert(task_id, task).is_some() {
            panic!("Task with same ID already exists in tasks");
        }

        let task_queue = self.task_queue.get(&priority);
        let waker = Arc::new(TaskWaker::new(
            task_id,
            task_queue.clone(),
            self.sleep_waker.clone(),
        ));

        self.tasks.get_mut(&task_id).unwrap().set_waker(waker);

        task_queue.lock().push_back(QueueItem::Poll(task_id));
    }

    pub fn get_spawner(&self) -> Spawner {
        Spawner::new(self.task_queue.clone())
    }

    fn get_next_task(&self) -> Option<QueueItem> {
        let mut next_task;
        next_task = self
            .task_queue
            .get(&TaskPriority::Interrupt)
            .lock()
            .pop_front();
        if let None = next_task {
            next_task = self
                .task_queue
                .get(&TaskPriority::Normal)
                .lock()
                .pop_front();
        }
        next_task
    }

    fn poll_task(&mut self, task_id: TaskID) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            // let waker = waker_cache
            //     .entry(task_id)
            //     .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));

            let waker = task.waker.as_ref().expect("Waker not set :/");
            let mut context = Context::from_waker(waker);
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    // Task done -> remove it and it's cached waker
                    self.tasks.remove(&task_id);
                    println!("Finishing task: {:?}", &task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn kill_task(&mut self, task_id: TaskID) {
        // Check if the task still exists
        if let Some(_) = self.tasks.get_mut(&task_id) {
            // "Kill" it
            self.tasks.remove(&task_id);
            println!("Killing task: {:?}", &task_id);
        }
    }

    pub fn run(&mut self) {
        'outer: loop {
            while let Some(item) = self.get_next_task() {
                match item {
                    QueueItem::Poll(id) => self.poll_task(id),
                    QueueItem::Spawn(task, priority) => self.spawn(task, priority),
                    QueueItem::Kill(id) => self.kill_task(id),
                }
                if self.tasks.is_empty() {
                    break 'outer;
                }
            }
            self.sleep_waker.sleep();
        }
    }
}

enum QueueItem {
    Spawn(Task, TaskPriority),
    Poll(TaskID),
    Kill(TaskID),
}

pub async fn async_yield_now() {
    YieldNow(true).await
}

struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 {
            // Will be ready next time
            self.0 = false;
            // Tell it we are ready to be woken again
            cx.waker().wake_by_ref();
            // Reschedule
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
