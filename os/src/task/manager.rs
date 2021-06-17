use super::task::TaskControlBlock;
use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Default)]
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}
