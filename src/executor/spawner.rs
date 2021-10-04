use futures_util::Future;

use super::{
    task::{Task, TaskID},
    QueueItem, TaskPriority, TaskQueue,
};

pub struct Spawner(TaskQueue);

impl Spawner {
    pub(super) fn new(handle: TaskQueue) -> Self {
        Self(handle)
    }

    pub fn spawn_task(&self, task: Task, priority: TaskPriority) {
        self.0
            .get(&priority)
            .lock()
            .push_back(QueueItem::Spawn(task, priority));
    }

    pub fn spawn(
        &self,
        future: impl Future<Output = ()> + Send + 'static,
        priority: TaskPriority,
    ) -> TaskID {
        let task = Task::new(future);
        let id = task.id;
        self.0
            .get(&priority)
            .lock()
            .push_back(QueueItem::Spawn(task, priority));
        id
    }

    pub fn kill(&self, id: TaskID) {
        self.0
            .get(&TaskPriority::Normal)
            .lock()
            .push_back(QueueItem::Kill(id))
    }
}

impl Clone for Spawner {
    fn clone(&self) -> Self {
        Spawner(self.0.clone())
    }
}
