use crate::config::*;
use crate::trap::TrapContext;
use core::{cell::RefCell, usize};
use lazy_static::lazy_static;

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
pub struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];
pub static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

pub trait Stack {
    fn get_sp(&self) -> usize;
}

impl Stack for KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
}

impl KernelStack {
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_prt = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_prt = cx;
        }
        unsafe { cx_prt.as_mut().unwrap() }
    }
}

impl Stack for UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
    }
    for i in 0..num_app {
        let base_i = get_base_i(i);
        (0..APP_SIZE_LIMIT)
            .map(|offset| offset + base_i)
            .for_each(|addr| unsafe {
                (addr as *mut u8).write_volatile(0);
            });
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
    }
}

fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

// * 用RefCell避免static mut
struct AppManager {
    inner: RefCell<AppManagerInner>,
}

struct AppManagerInner {
    num_app: usize,
    current_app: usize,
}

unsafe impl Sync for AppManager {}

impl AppManagerInner {
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: AppManager = AppManager {
        inner: RefCell::new({
            extern "C" {
                fn _num_app();
            }
            let num_app = unsafe { (_num_app as usize as *const usize).read_volatile() };
            AppManagerInner {
                num_app,
                current_app: 0,
            }
        }),
    };
}

pub fn run_next_app() -> ! {
    let current_app = APP_MANAGER.inner.borrow().get_current_app();
    APP_MANAGER.inner.borrow_mut().move_to_next_app();
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(
            KERNEL_STACK[current_app].push_context(TrapContext::app_init_context(
                get_base_i(current_app),
                USER_STACK[current_app].get_sp(),
            )) as *const _ as usize,
        );
    }
    panic!("Unreachable in batch::run_current_app!");
}
