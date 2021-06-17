const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;
use core::usize;

use crate::{
    mm::translated_byte_buffer,
    sbi::console_getchar,
    task::{current_user_token, suspend_current_and_run_next},
};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    if FD_STDOUT == fd {
        let buffers = translated_byte_buffer(current_user_token(), buf, len);
        for buffer in buffers {
            print!("{}", core::str::from_utf8(buffer).unwrap());
        }
        return len as isize;
    }
    -1
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    if FD_STDIN == fd {
        assert_eq!(len, 1, "Only support len = 1 in sys_read");
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        let mut buffers = translated_byte_buffer(current_user_token(), buf, 1);
        unsafe {
            buffers[0].as_mut_ptr().write_volatile(ch);
        }
        return 1;
    }
    -1
}
