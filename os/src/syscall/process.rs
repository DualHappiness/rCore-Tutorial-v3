use core::iter::Map;

use crate::{config::MAX_ALLOC_SIZE, mm::{translated, translated_byte_buffer, MapPermission}, task::{
        current_user_token, exit_current_and_run_next, set_priority, suspend_current_and_run_next,
    }, timer::{get_time_ms, TimeVal}};

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
        let time = translated(current_user_token(), time);
        *time = get_time_ms();
    }
    0
}

pub fn sys_set_priority(priority: isize) -> isize {
    set_priority(priority)
}

pub fn mmap(start: usize, len: usize, port: usize) -> i32 {
    if port >> 3 != 0 || len > MAX_ALLOC_SIZE {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut perm = MapPermission::empty();
    if port & (1 << 0) != 0 {
        perm |= MapPermission::R;
    }
    if port & (1 << 1) != 0 {
        perm |= MapPermission::W;
    }
    if port & (1 << 2) != 0 {
        perm |= MapPermission::X;
    }

    return 0;
}
