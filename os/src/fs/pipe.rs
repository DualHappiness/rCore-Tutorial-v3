use alloc::sync::{Arc, Weak};
use spin::Mutex;

use crate::task::suspend_current_and_run_next;

use super::File;

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq, Debug)]
enum RingBufferStatus {
    FULL,
    EMPTY,
    NORMAL,
}

impl Default for RingBufferStatus {
    fn default() -> Self {
        RingBufferStatus::EMPTY
    }
}

#[derive(Default, Debug)]
pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }
}
impl PipeRingBuffer {
    pub fn read_byte(&mut self) -> u8 {
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;

        self.status = if self.head == self.tail {
            RingBufferStatus::EMPTY
        } else {
            RingBufferStatus::NORMAL
        };
        c
    }
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::EMPTY {
            0
        } else {
            if self.tail > self.head {
                self.tail - self.head
            } else {
                self.tail + RING_BUFFER_SIZE - self.head
            }
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        self.status = if self.tail == self.head {
            RingBufferStatus::FULL
        } else {
            RingBufferStatus::NORMAL
        };
    }
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::FULL {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

#[derive(Debug)]
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }
    pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

impl File for Pipe {
    fn read(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.readable);
        let mut buf_iter = buf.into_iter();
        let mut read_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return read_size;
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe { *byte_ref = ring_buffer.read_byte() }
                    read_size += 1;
                } else {
                    return read_size;
                }
            }
        }
    }

    fn write(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.writable);
        let mut buf_iter = buf.into_iter();
        let mut write_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    write_size += 1;
                } else {
                    return write_size;
                }
            }
        }
    }
}

pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.lock().set_write_end(&write_end);
    (read_end, write_end)
}
