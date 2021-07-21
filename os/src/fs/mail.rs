use spin::Mutex;

use super::File;

// ! 这地方大了之后可能会导致在换页 但内核换页没处理 所以会有一定的问题
const MAX_SLOT_SIZE: usize = 8;
const MAX_MAIL_SIZE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
enum RingBufferStatus {
    Empty,
    Full,
    Normal,
}
impl Default for RingBufferStatus {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Clone, Copy)]
struct Mail {
    arr: [u8; MAX_MAIL_SIZE],
    len: usize,
}
impl Default for Mail {
    fn default() -> Self {
        Self {
            arr: [0; MAX_MAIL_SIZE],
            len: 0,
        }
    }
}

#[derive(Debug, Default)]
struct RingBuffer {
    arr: [Mail; MAX_SLOT_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
}

impl RingBuffer {
    pub fn is_readable(&self) -> bool {
        self.status != RingBufferStatus::Empty
    }
    pub fn is_writable(&self) -> bool {
        self.status != RingBufferStatus::Full
    }
    pub fn read(&mut self) -> Mail {
        let mail = self.arr[self.head];
        self.head = (self.head + 1) % MAX_SLOT_SIZE;
        self.status = if self.head == self.tail {
            RingBufferStatus::Empty
        } else {
            RingBufferStatus::Normal
        };
        mail
    }
    pub fn write(&mut self, mail: Mail) {
        self.arr[self.tail] = mail;
        self.tail = (self.tail + 1) % MAX_SLOT_SIZE;
        self.status = if self.head == self.tail {
            RingBufferStatus::Full
        } else {
            RingBufferStatus::Normal
        };
    }
    pub fn clear(&mut self) {
        self.status = RingBufferStatus::Empty;
        self.head = 0;
        self.tail = 0;
    }
}

// mpsc so write index has lock
#[derive(Debug, Default)]
pub struct MailList {
    buffers: Mutex<RingBuffer>,
}

impl MailList {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn is_readable(&self) -> bool {
        self.buffers.lock().is_readable()
    }
    pub fn is_writable(&self) -> bool {
        self.buffers.lock().is_writable()
    }
    pub fn clear(&self) {
        self.buffers.lock().clear();
    }
}

// TODO copy from slice
impl File for MailList {
    fn read(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.is_readable());
        let mut buf_iter = buf.into_iter();
        let mut read_size = 0usize;

        let mail = self.buffers.lock().read();
        for i in 0..mail.len {
            if let Some(byte_ref) = buf_iter.next() {
                unsafe { *byte_ref = mail.arr[i] }
                read_size += 1;
            } else {
                break;
            }
        }
        return read_size;
    }

    fn write(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.is_writable());
        let mut buf_iter = buf.into_iter();
        let mut write_size = 0usize;
        let mut mail = Mail::default();
        while let Some(byte_ref) = buf_iter.next() {
            mail.arr[write_size] = unsafe { *byte_ref };
            write_size += 1;
            if write_size >= MAX_MAIL_SIZE {
                break;
            }
        }
        mail.len = write_size;
        self.buffers.lock().write(mail);
        write_size
    }

    fn readable(&self) -> bool {
        self.is_readable()
    }

    fn writable(&self) -> bool {
        self.is_writable()
    }

    fn fstat(&self, _st: &mut easy_fs::Stat) -> isize {
        -1
    }
}
