use core::ops::{Add, AddAssign};

use crate::config::BIG_STRIDE;

#[derive(Debug, Clone, Copy)]
pub struct Stride(u8);

impl Eq for Stride {}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some((self.0 - other.0).cmp(&(BIG_STRIDE / 2)))
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0 - other.0) == BIG_STRIDE / 2
    }
}

impl Add<u8> for Stride {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u8> for Stride {
    fn add_assign(&mut self, rhs: u8) {
        self.0 += rhs
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TaskControlBlock {
    pub task_cx_ptr: usize,
    pub task_status: TaskStatus,
    pub stride: Stride,
    pub total_stride: usize,
    // pub pass: usize,
    pub priority: u8,
}

impl TaskControlBlock {
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
}

impl Default for TaskControlBlock {
    fn default() -> Self {
        Self {
            task_cx_ptr: 0,
            task_status: TaskStatus::UnInit,
            stride: Stride(0),
            total_stride: 0,
            priority: 16,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
