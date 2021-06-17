use alloc::sync::Arc;

use crate::{
    config::MAX_ALLOC_SIZE,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str, MapPermission},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::{get_time_ms, TimeVal},
};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit");
    // crate::batch::run_next_app()
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(time: *mut TimeVal) -> isize {
    todo!()
    // let time = translated(current_user_token(), time);
    // *time = get_time_ms();
    // 0
}

pub fn sys_getpid() -> isize {
    todo!()
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();

    let mut inner = task.acquire_inner_lock();
    let mut ret = -2;
    for (index, child) in inner.children.iter().enumerate() {
        if pid == -1 || pid as usize == child.getpid() {
            ret = -1;
            if child.acquire_inner_lock().is_zombie() {
                let child = inner.children.remove(index);
                assert_eq!(Arc::strong_count(&child), 1);
                let found_pid = child.getpid();
                let exit_code = child.acquire_inner_lock().exit_code;
                *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
                ret = found_pid as isize;
                break;
            }
        }
    }
    ret
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    println!("begin fork. ");
    let new_task = current_task.fork();
    println!("after fork. ");
    let new_pid = new_task.pid.0;

    // 根据fork线程的返回值 区分自己是子线程还是父线程
    let tarp_cx = new_task.acquire_inner_lock().get_trap_cx();
    // a0
    tarp_cx.x[10] = 0;

    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    println!("exec path : {:?}", path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

pub fn sys_set_priority(priority: isize) -> isize {
    // set_priority(priority)
    todo!()
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    if prot & !0x7 != 0 || prot & 0x7 == 0 || len > MAX_ALLOC_SIZE || start % 0x1000 != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let perm = MapPermission::from_bits_truncate((prot as u8) << 1);
    // crate::task::alloc(start, len, perm).map_or(-1, |size| size as isize)
    todo!()
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if len > MAX_ALLOC_SIZE || start % 0x1000 != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    todo!()
    // crate::task::dealloc(start, len).map_or(-1, |size| size as isize)
}
