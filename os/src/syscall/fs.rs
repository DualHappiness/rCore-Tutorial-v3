const FD_STDOUT: usize = 1;
use core::usize;

use crate::{mm::translated_byte_buffer, task::current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    if let FD_STDOUT = fd {
        let buffers = translated_byte_buffer(current_user_token(), buf, len);
        for buffer in buffers {
            print!("{}", core::str::from_utf8(buffer).unwrap());
        }
        return len as isize;
    }
    -1
}
