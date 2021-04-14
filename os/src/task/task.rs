#[derive(Clone, Copy, Debug)]
pub struct TaskControlBlock {
    pub task_cx_ptr: usize,
    pub task_status: TaskStatus,
    pub stride: usize,
    // pub pass: usize,
    pub priority: usize,
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
            stride: 0,
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
