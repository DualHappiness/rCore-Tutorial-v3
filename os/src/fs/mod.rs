use core::fmt::Debug;

use crate::mm::UserBuffer;

mod mail;
mod pipe;
mod stdio;

pub use mail::MailList;
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};

pub trait File: Send + Sync + Debug {
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
