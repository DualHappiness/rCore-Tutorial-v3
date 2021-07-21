use crate::drivers::BLOCK_DEVICE;
use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::fmt::Debug;
use easy_fs::{EasyFileSystem, Inode, Stat};
use lazy_static::lazy_static;
use spin::Mutex;

use super::File;

#[derive(Debug)]
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: Mutex<OSInodeInner>,
}
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}
impl Debug for OSInodeInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "offset: {}", self.offset)
    }
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner { offset: 0, inode }),
        }
    }
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

impl File for OSInode {
    fn read(&self, mut buf: crate::mm::UserBuffer) -> usize {
        let mut inner = self.inner.lock();
        let mut read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let temp_size = inner.inode.read_at(inner.offset, *slice);
            if temp_size == 0 {
                break;
            }
            read_size += temp_size;
            inner.offset += temp_size;
        }
        read_size
    }

    fn write(&self, buf: crate::mm::UserBuffer) -> usize {
        let mut inner = self.inner.lock();
        let mut write_size = 0usize;
        for slice in buf.buffers.iter() {
            let temp_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(temp_size, slice.len());
            inner.offset += temp_size;
            write_size += temp_size;
        }
        write_size
    }

    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn fstat(&self, st: &mut Stat) -> isize {
        let inner = self.inner.lock();
        ROOT_INODE.fstat(&inner.inode, st) as isize
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    pub struct OpenFlags: u32 {
        const READ_ONLY = 0;
        const WRITE_ONLY = 1 << 0;
        const READ_WRITE = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}
impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRITE_ONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    if flags.contains(OpenFlags::READ_ONLY | OpenFlags::READ_WRITE) {
        return None;
    }

    let (readable, writable) = flags.read_write();
    match ROOT_INODE.find(name) {
        None => {
            if flags.contains(OpenFlags::CREATE) {
                ROOT_INODE
                    .create(name)
                    .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
            } else {
                None
            }
        }
        Some(inode) => {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        }
    }
}

pub fn linkat(oldpath: &str, newpath: &str, _flag: OpenFlags) -> bool {
    if oldpath == newpath {
        return false;
    }
    ROOT_INODE.link(oldpath, newpath)
}

pub fn unlinkat(path: &str) -> bool {
    ROOT_INODE.unlink(path)
}
