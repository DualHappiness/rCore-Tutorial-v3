mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

pub use manager::add_task;
pub use processor::{current_task, current_trap_cx, current_user_token, run_tasks};
use task::TaskStatus;

use self::{
    processor::{schedule, take_current_task},
    task::TaskControlBlock,
};
use crate::loader::{get_app_data, get_app_data_by_name};
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::lazy_static;

type Task = Arc<TaskControlBlock>;

lazy_static! {
    pub static ref INITPROC: Task = Arc::new(TaskControlBlock::new(
        // get_app_data_by_name("ch2_hello_world").unwrap()
        get_app_data(1)
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

// pub fn run_first_task() {
//     TASK_MANAGER.run_first_task();
// }

// fn run_next_task() {
//     TASK_MANAGER.run_next_task();
// }

// fn mark_current_suspended() {
//     TASK_MANAGER.mark_current_suspended();
// }

// fn mark_current_exited() {
//     TASK_MANAGER.mark_current_exited();
// }

pub fn suspend_current_and_run_next() {
    println!("suspend\n");
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

// pub fn set_priority(priority: isize) -> isize {
//     TASK_MANAGER.set_priority(priority)
// }

// pub fn current_trap_cx() -> &'static mut TrapContext {
//     TASK_MANAGER.get_current_trap_cx()
// }

// impl TaskManager {
//     fn get_current_token(&self) -> usize {
//         let inner = self.inner.borrow();
//         let current = inner.current_task;
//         inner.tasks[current].get_user_token()
//     }
//     fn get_current_trap_cx(&self) -> &mut TrapContext {
//         let inner = self.inner.borrow();
//         let current = inner.current_task;
//         inner.tasks[current].get_trap_cx()
//     }

//     fn alloc(&self, start: usize, len: usize, perm: MapPermission) -> Option<usize> {
//         let mut inner = self.inner.borrow_mut();
//         let current = inner.current_task;
//         inner.tasks[current].memory_set.alloc(start, len, perm)
//     }
//     fn dealloc(&self, start: usize, len: usize) -> Option<usize> {
//         let mut inner = self.inner.borrow_mut();
//         let current = inner.current_task;
//         inner.tasks[current].memory_set.dealloc(start, len)
//     }
// }

// pub fn alloc(start: usize, len: usize, perm: MapPermission) -> Option<usize> {
//     TASK_MANAGER.alloc(start, len, perm)
// }

// pub fn dealloc(start: usize, len: usize) -> Option<usize> {
//     TASK_MANAGER.dealloc(start, len)
// }
