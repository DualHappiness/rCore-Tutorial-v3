use crate::{
    task::{exit_current_and_run_next, set_priority, suspend_current_and_run_next},
    timer::{get_time_ms, TimeVal},
};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit");
    // crate::batch::run_next_app()
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(time: *mut TimeVal) -> isize {
    unsafe {
        *time = get_time_ms();
    }
    0
}

pub fn sys_set_priority(priority: isize) -> isize {
    set_priority(priority)
}
