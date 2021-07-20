use core::fmt::Debug;

use crate::mm::UserBuffer;

mod inode;
mod mail;
mod pipe;
mod stdio;

pub use inode::{list_apps, open_file, OpenFlags};
pub use mail::MailList;
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};

pub trait File: Send + Sync + Debug {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
