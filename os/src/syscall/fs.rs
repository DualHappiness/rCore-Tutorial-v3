use core::usize;

use easy_fs::Stat;
use log::info;

use crate::{
    fs::{linkat, make_pipe, open_file, unlinkat, File, OpenFlags},
    mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer},
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

pub fn sys_open(_dirfd: i32, path: *const u8, flags: u32, _mode: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.acquire_inner_lock();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd > inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(inner.fd_table[fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_linkat(
    _olddirfd: i32,
    oldpath: *const u8,
    _newdirfd: i32,
    newpath: *const u8,
    flags: u32,
) -> isize {
    let token = current_user_token();
    let oldpath = translated_str(token, oldpath);
    let newpath = translated_str(token, newpath);
    info!(
        "sys link old: {}, new: {}, flags: {}",
        oldpath, newpath, flags
    );
    linkat(
        oldpath.as_str(),
        newpath.as_str(),
        OpenFlags::from_bits(flags).unwrap(),
    ) as isize
}
pub fn sys_unlinkat(_dirfd: i32, path: *const u8, _flags: u32) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    info!("sys unlink path: {}", path);
    unlinkat(path.as_str()) as isize
}
pub fn sys_fstat(fd: i32, st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let st = translated_refmut(token, st);
    let fd = fd as usize;
    let inner = task.acquire_inner_lock();
    if fd < inner.fd_table.len() {
        if let Some(file) = &inner.fd_table[fd] {
            return file.fstat(st);
        }
    }
    -1
}
