use crate::timer::TimeVal;

const SYSCALL_DUP: usize = 24;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_MAILREAD: usize = 401;
const SYSCALL_MAILWRITE: usize = 402;

mod fs;
mod process;

use easy_fs::Stat;
use fs::*;
use log::debug;
use process::*;

pub fn syscall(syscall_id: usize, args: [usize; 5]) -> isize {
    debug!("system call : {}", syscall_id);
    match syscall_id {
        // io
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_MAILREAD => sys_mailread(args[0] as *const u8, args[1]),
        SYSCALL_MAILWRITE => sys_mailwrite(args[0], args[1] as *const u8, args[2]),

        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal),

        SYSCALL_DUP => sys_dup(args[0] as usize),
        SYSCALL_OPEN => sys_open(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as u32,
            args[3] as u32,
        ),
        SYSCALL_LINKAT => sys_linkat(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as i32,
            args[3] as *const u8,
            args[4] as u32,
        ),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SYSCALL_FSTAT => sys_fstat(args[0] as i32, args[1] as *mut Stat),
        // process
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        // priority
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as isize),
        // mmap
        SYSCALL_MMAP => sys_mmap(args[0] as usize, args[1] as usize, args[2] as usize),
        SYSCALL_MUNMAP => sys_munmap(args[0] as usize, args[1] as usize),
        //
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
