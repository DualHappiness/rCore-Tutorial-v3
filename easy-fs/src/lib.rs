#![no_std]
#![feature(llvm_asm)]

extern crate alloc;
extern crate log;

mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;

pub const BLOCK_SIZE: usize = 512;
type DataBlock = [u8; BLOCK_SIZE];
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
pub use vfs::{Inode, Stat, StatMode};
