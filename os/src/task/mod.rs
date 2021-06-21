mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

pub use manager::{add_task, get_maillist, add_mailist};
pub use processor::{current_task, current_trap_cx, current_user_token, run_tasks};
use task::TaskStatus;

use self::{
    processor::{schedule, take_current_task},
    task::TaskControlBlock,
};
use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::lazy_static;

type Task = Arc<TaskControlBlock>;

lazy_static! {
    pub static ref INITPROC: Task = {
        let name = option_env!("ENTRY").unwrap();
        Arc::new(TaskControlBlock::new(get_app_data_by_name(name).unwrap()))
    };
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();

    let mut task_inner = task.acquire_inner_lock();
    let task_cx_ptr2 = task_inner.get_task_cx_ptr2();

    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    add_task(task);
    schedule(task_cx_ptr2);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    let mut initproc_inner = INITPROC.acquire_inner_lock();
    for child in &inner.children {
        child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone());
    }
    drop(initproc_inner);

    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);

    drop(task);

    let _unused: usize = 0;
    schedule(&_unused as *const _);
}

pub fn spawn(parent: &Arc<TaskControlBlock>, elf_data: &[u8]) -> Arc<TaskControlBlock> {
    TaskControlBlock::spawn(parent, elf_data)
}
