use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use log::debug;
use core::ops::{Add, AddAssign};
use spin::{Mutex, MutexGuard};

use crate::mm::translated_refmut;
use crate::{
    config::{BIG_STRIDE, MAX_PRIORITY, TRAP_CONTEXT},
    fs::{File, Stdin, Stdout},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    trap::{trap_handler, TrapContext},
};

use super::{
    add_mailist,
    pid::{pid_alloc, KernelStack, PidHandle},
    TaskContext,
};

#[derive(Debug, Clone, Copy, Default)]
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
pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub task_cx_ptr: usize,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,

    // priority about
    pub stride: Stride,
    pub total_stride: usize,
    // pub pass: usize,
    pub priority: u8,

    //
    pub fd_table: Vec<Option<Arc<dyn File>>>,
}

impl Default for TaskControlBlockInner {
    fn default() -> Self {
        Self {
            trap_cx_ppn: Default::default(),
            base_size: Default::default(),
            task_cx_ptr: Default::default(),
            task_status: TaskStatus::Ready,
            memory_set: Default::default(),
            parent: None,
            children: Vec::new(),
            exit_code: 0,
            stride: Stride(0),
            total_stride: 0,
            priority: 16,
            fd_table: vec![
                Some(Arc::new(Stdin)),
                Some(Arc::new(Stdout)),
                Some(Arc::new(Stdout)),
            ],
        }
    }
}

impl TaskControlBlockInner {
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    pub fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlockInner {
    pub fn alloc_fd(&mut self) -> usize {
        match self
            .fd_table
            .iter()
            .enumerate()
            .find(|(_, file)| file.is_none())
        {
            Some((fd, _)) => fd,
            None => {
                self.fd_table.push(None);
                self.fd_table.len() - 1
            }
        }
    }
}

// ! 这里千万不能使用default来图方便 因为内部实现了drop的字段会把自己释放调
#[derive(Debug)]
pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    inner: Mutex<TaskControlBlockInner>,
}

impl TaskControlBlock {
    fn new_block(mut inner: TaskControlBlockInner) -> (Self, usize) {
        let pid = pid_alloc();
        let kernel_stack = KernelStack::new(&pid);

        let kernel_stack_top = kernel_stack.get_top();
        let task_cx_ptr = kernel_stack.push_on_top(TaskContext::goto_trap_return());
        inner.task_cx_ptr = task_cx_ptr as usize;
        inner.priority = 16;
        let task_control_block = Self {
            pid,
            kernel_stack,
            inner: Mutex::new(inner),
        };
        add_mailist(task_control_block.pid.0);
        (task_control_block, kernel_stack_top)
    }
    pub fn new(elf_data: &[u8]) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let (task_control_block, kernel_stack_top) = Self::new_block(TaskControlBlockInner {
            trap_cx_ppn,
            base_size: user_sp,
            memory_set,
            ..Default::default()
        });
        let trap_cx = task_control_block.acquire_inner_lock().get_trap_cx();
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

impl TaskControlBlock {
    pub fn acquire_inner_lock(&self) -> MutexGuard<TaskControlBlockInner> {
        debug!(
            "pid: {} acquire lock {:?}.",
            self.pid.0,
            self.inner.is_locked()
        );
        self.inner.lock()
    }
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}

impl TaskControlBlock {
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        let (memory_set, mut user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // arguments 先保存arg指针数组 然后再挨个压栈
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    memory_set.token(),
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(memory_set.token(), p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(memory_set.token(), p as *mut u8) = 0;
        }
        // alignment
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let mut inner = self.acquire_inner_lock();
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
    }
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.acquire_inner_lock();
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let new_fd_table = parent_inner.fd_table.clone();
        let (tcb_inner, kernel_stack_top) = Self::new_block(TaskControlBlockInner {
            trap_cx_ppn,
            base_size: parent_inner.base_size,
            memory_set,
            parent: Some(Arc::downgrade(self)),
            fd_table: new_fd_table,
            ..Default::default()
        });
        let task_control_block = Arc::new(tcb_inner);
        parent_inner.children.push(task_control_block.clone());

        let trap_cx = task_control_block.acquire_inner_lock().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }
    pub fn spawn(self: &Arc<Self>, elf_data: &[u8]) -> Arc<Self> {
        let mut parent_inner = self.acquire_inner_lock();
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let (tcb_inner, kernel_stack_top) = Self::new_block(TaskControlBlockInner {
            trap_cx_ppn,
            base_size: parent_inner.base_size,
            memory_set,
            parent: Some(Arc::downgrade(self)),
            ..Default::default()
        });
        let task_control_block = Arc::new(tcb_inner);
        parent_inner.children.push(task_control_block.clone());

        let trap_cx = task_control_block.acquire_inner_lock().get_trap_cx();
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

impl TaskControlBlock {
    pub fn set_priority(&self, priority: isize) -> isize {
        if priority < MAX_PRIORITY as isize {
            -1
        } else {
            self.acquire_inner_lock().priority = priority.max(u8::MAX as isize) as u8;
            priority
        }
    }
}

impl TaskControlBlock {
    pub fn alloc(&self, start: usize, len: usize, perm: MapPermission) -> Option<usize> {
        self.acquire_inner_lock().memory_set.alloc(start, len, perm)
    }
    pub fn dealloc(&self, start: usize, len: usize) -> Option<usize> {
        self.acquire_inner_lock().memory_set.dealloc(start, len)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Ready
    }
}
