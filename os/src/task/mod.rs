use crate::{
    config::{BIG_STRIDE, MAX_PRIORITY, MAX_STRIDE},
    loader::get_app_data,
    mm::MapPermission,
};
use crate::{loader::get_num_app, trap::TrapContext};
use alloc::vec::Vec;
use core::{cell::RefCell, usize};
use lazy_static::lazy_static;
use task::{TaskControlBlock, TaskStatus};

use self::switch::__switch;
mod context;
mod switch;
mod task;

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

impl TaskManager {
    #[allow(unused)]
    fn get_current(&self) -> usize {
        self.inner.borrow().current_task
    }

    fn set_priority(&self, priority: isize) -> isize {
        if priority >= MAX_PRIORITY as isize {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[current].priority = priority as u8;
            priority
        } else {
            -1
        }
    }

    fn run_first_task(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.tasks[0].task_status = TaskStatus::Running;
        let next_task_cx_ptr2 = inner.tasks[0].get_task_cx_ptr2();
        let _unused: usize = 0;
        core::mem::drop(inner);
        unsafe {
            __switch(&_unused as *const _, next_task_cx_ptr2);
        }
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .filter(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
            .min_by(|a, b| inner.tasks[*a].stride.cmp(&inner.tasks[*b].stride))
    }

    #[allow(unused)]
    fn kill_deadloop_task(&self) {
        let mut inner = self.inner.borrow_mut();

        inner
            .tasks
            .as_mut_slice()
            .into_iter()
            .filter(|task| task.task_status == TaskStatus::Ready)
            .filter(|task| task.total_stride >= MAX_STRIDE)
            .for_each(|task| task.task_status = TaskStatus::Exited)
    }

    fn run_next_task(&self) {
        // self.kill_deadloop_task();
        // println!("the next task is : {:?}", self.find_next_task());
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            let pass = BIG_STRIDE / inner.tasks[next].priority;
            inner.tasks[next].stride += pass;
            inner.tasks[next].total_stride += pass as usize;
            inner.current_task = next;
            let current_task_cx_ptr2 = inner.tasks[current].get_task_cx_ptr2();
            let next_task_cx_ptr2 = inner.tasks[next].get_task_cx_ptr2();
            core::mem::drop(inner);
            unsafe {
                __switch(current_task_cx_ptr2, next_task_cx_ptr2);
            }
        } else {
            panic!("All application completed!");
        }
    }
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app={}", num_app);
        let tasks: Vec<_> = (0..num_app)
            .map(|i| TaskControlBlock::new(get_app_data(i), i))
            .collect();
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }),
        }
    };
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn get_current() -> usize {
    TASK_MANAGER.get_current()
}

pub fn set_priority(priority: isize) -> isize {
    TASK_MANAGER.set_priority(priority)
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

impl TaskManager {
    fn get_current_token(&self) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }
    fn get_current_trap_cx(&self) -> &mut TrapContext {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_trap_cx()
    }

    fn alloc(&self, start: usize, len: usize, perm: MapPermission) -> Option<usize> {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].memory_set.alloc(start, len, perm)
    }
    fn dealloc(&self, start: usize, len: usize) -> Option<usize> {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].memory_set.dealloc(start, len)
    }
}

pub fn alloc(start: usize, len: usize, perm: MapPermission) -> Option<usize> {
    TASK_MANAGER.alloc(start, len, perm)
}

pub fn dealloc(start: usize, len: usize) -> Option<usize> {
    TASK_MANAGER.dealloc(start, len)
}