use core::usize;

use crate::{
    fs::{make_pipe, File},
    mm::{translated_byte_buffer, translated_refmut, UserBuffer},
    task::{current_task, current_user_token, get_maillist},
};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd < inner.fd_table.len() {
        if let Some(file) = &inner.fd_table[fd] {
            let file = file.clone();
            // 可能会潜在的进程切换 所以要提前释放
            drop(inner);
            if let Some(buffers) = translated_byte_buffer(token, buf, len) {
                return file.write(UserBuffer::new(buffers)) as isize;
            }
        }
    }
    -1
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd < inner.fd_table.len() {
        if let Some(file) = &inner.fd_table[fd] {
            let file = file.clone();
            // 可能会潜在的进程切换 所以要提前释放
            drop(inner);
            if let Some(buffers) = translated_byte_buffer(token, buf, len) {
                return file.read(UserBuffer::new(buffers)) as isize;
            }
        }
    }
    -1
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd < inner.fd_table.len() && inner.fd_table[fd].is_some() {
        inner.fd_table[fd].take();
        return 0;
    }
    return -1;
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_mailread(buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let pid = current_task().unwrap().getpid();
    let mail_list = get_maillist(pid);
    if mail_list.is_readable() {
        if len == 0 {
            return 0;
        }
        if let Some(buffers) = translated_byte_buffer(token, buf, len) {
            return mail_list.read(UserBuffer::new(buffers)) as isize;
        }
    }
    -1
}

pub fn sys_mailwrite(pid: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let mail_list = get_maillist(pid);
    if mail_list.is_writable() {
        if len == 0 {
            return 0;
        }

        if let Some(buffers) = translated_byte_buffer(token, buf, len) {
            return mail_list.write(UserBuffer::new(buffers)) as isize;
        }
    }
    -1
}
