use crate::{config::MAX_STRIDE, fs::MailList};
use alloc::vec::Vec;

use super::task::TaskControlBlock;
use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

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
        self.kill_deadloop_task();
        match self
            .ready_queue
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.acquire_inner_lock()
                    .stride
                    .cmp(&b.acquire_inner_lock().stride)
            }) {
            None => None,
            Some((index, _)) => self.ready_queue.swap_remove_front(index),
        }
    }

    fn kill_deadloop_task(&mut self) {
        self.ready_queue
            .retain(|task| task.acquire_inner_lock().total_stride < MAX_STRIDE);
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

pub struct MailManager {
    mail_lists: Vec<Arc<MailList>>,
}
unsafe impl Sync for MailManager {}
impl MailManager {
    pub fn new() -> Self {
        Self {
            mail_lists: Vec::new(),
        }
    }
}

lazy_static! {
    pub static ref MAIL_MANAGER: RwLock<MailManager> = RwLock::new(MailManager::new());
}

pub fn get_maillist(pid: usize) -> Arc<MailList> {
    MAIL_MANAGER.read().mail_lists[pid].clone()
}

pub fn add_mailist(pid: usize) {
    println!("mail pid is {:#x}", pid);
    let mut manager = MAIL_MANAGER.write();
    if manager.mail_lists.len() < pid + 1 {
        while manager.mail_lists.len() < pid + 1 {
            manager.mail_lists.push(Arc::new(MailList::new()));
        }
    }
    manager.mail_lists[pid].clear();
}
