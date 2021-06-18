use core::cell::RefCell;

use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{config::BIG_STRIDE, trap::TrapContext};

use super::{add_task, manager::fetch_task, switch::__switch, task::{TaskControlBlock, TaskStatus}};
type Task = Arc<TaskControlBlock>;

#[derive(Default)]
pub struct Processor {
    inner: RefCell<ProcessorInner>,
}

unsafe impl Sync for Processor {}

#[derive(Default)]
pub struct ProcessorInner {
    current: Option<Task>,
    idle_task_cx_ptr: usize,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_task_cx_ptr: 0,
            }),
        }
    }
}

lazy_static! {
    // pre processor pre thread, so no need for mutex
    pub static ref PROCESSOR: Processor = Processor::new();
}

impl Processor {
    pub fn take_current(&self) -> Option<Task> {
        self.inner.borrow_mut().current.take()
    }
    pub fn current(&self) -> Option<Task> {
        self.inner.borrow().current.as_ref().map(|task| Arc::clone(task))
        // self.inner.borrow().current.clone()
    }
}

pub fn take_current_task() -> Option<Task> {
    PROCESSOR.take_current()
}
pub fn current_task() -> Option<Task> {
    PROCESSOR.current()
}
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.acquire_inner_lock().get_user_token();
    token
}
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_cx()
}

impl Processor {
    fn get_idle_task_cx_prt2(&self) -> *const usize {
        let inner = self.inner.borrow();
        &inner.idle_task_cx_ptr as *const usize
    }
    pub fn run(&self) {
        loop {
            if let Some(task) = fetch_task() {
                let idle_task_cx_ptr2 = self.get_idle_task_cx_prt2();

                let mut task_inner = task.acquire_inner_lock();
                let next_task_cx_ptr2 = task_inner.get_task_cx_ptr2();
                task_inner.task_status = TaskStatus::Running;
                let pass = BIG_STRIDE / task_inner.priority;
                task_inner.stride += pass;
                task_inner.total_stride += pass as usize;
                drop(task_inner);

                self.inner.borrow_mut().current = Some(task);
                unsafe { __switch(idle_task_cx_ptr2, next_task_cx_ptr2) }
            }
        }
    }
}

pub fn run_tasks() {
    PROCESSOR.run();
}
pub fn schedule(switched_task_cx_ptr2: *const usize) {
    let idle_task_cx_ptr2 = PROCESSOR.get_idle_task_cx_prt2();
    unsafe { __switch(switched_task_cx_ptr2, idle_task_cx_ptr2) }
}
