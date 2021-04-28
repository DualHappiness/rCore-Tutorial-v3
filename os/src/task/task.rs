use core::ops::{Add, AddAssign};

use crate::{
    config::{kernel_stack_position, BIG_STRIDE, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    trap::{trap_handler, TrapContext},
};

use super::TaskContext;

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

#[derive(Debug)]
pub struct TaskControlBlock {
    pub task_cx_ptr: usize,
    pub task_status: TaskStatus,
    pub stride: Stride,
    pub total_stride: usize,
    // pub pass: usize,
    pub priority: u8,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        crate::mm::KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_cx_prt =
            (kernel_stack_top - core::mem::size_of::<TaskContext>()) as *mut TaskContext;
        unsafe {
            *task_cx_prt = TaskContext::goto_trap_return();
        }
        let task_control_block = Self {
            task_cx_ptr: task_cx_prt as usize,
            task_status,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            ..Self::default()
        };
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
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
            memory_set: MemorySet::new_bare(),
            trap_cx_ppn: 0.into(),
            base_size: 0,
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
