use crate::{
    config::MAX_ALLOC_SIZE,
    mm::{translated, MapPermission},
    task::{
        current_user_token, exit_current_and_run_next, set_priority, suspend_current_and_run_next,
    },
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
    let time = translated(current_user_token(), time);
    *time = get_time_ms();
    0
}

pub fn sys_set_priority(priority: isize) -> isize {
    set_priority(priority)
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    if prot & !0x7 != 0 || prot & 0x7 == 0 || len > MAX_ALLOC_SIZE || start % 0x1000 != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let perm = MapPermission::from_bits_truncate((prot as u8) << 1);
    crate::task::alloc(start, len, perm).map_or(-1, |size| size as isize)
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if len > MAX_ALLOC_SIZE || start % 0x1000 != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    crate::task::dealloc(start, len).map_or(-1, |size| size as isize)
}
