const FD_STDOUT: usize = 1;
use core::usize;

use crate::loader::{Stack, USER_STACK};
use crate::{config::*, task::get_current};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    if let FD_STDOUT = fd {
        let start = buf as usize;
        let current = get_current();
        let sp = USER_STACK[current].get_sp();

        if (start >= APP_BASE_ADDRESS && start + len <= APP_BASE_ADDRESS + APP_SIZE_LIMIT)
            || (start >= sp - USER_STACK_SIZE && start + len <= sp)
        {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            return len as isize;
        }
    }
    -1
}
