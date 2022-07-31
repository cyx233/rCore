use core::cmp::min;

use super::File;
use crate::mm::UserBuffer;
use alloc::{collections::VecDeque, sync::Arc};
use spin::Mutex;

pub struct MailBox {
    buffer: Arc<Mutex<MailRingBuffer>>,
}

impl MailBox {
    pub fn new() -> Self {
        let buffer = Arc::new(Mutex::new(MailRingBuffer::new()));
        Self { buffer }
    }
}

const RING_BUFFER_SIZE: usize = 4096;

pub struct MailRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    len_vec: VecDeque<usize>,
}

impl MailRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            len_vec: VecDeque::new(),
        }
    }
    pub fn write_datagram(&mut self, buf: UserBuffer) -> usize {
        let len = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut write_size = 0;
        let begin = self.tail * 256;
        self.tail = (self.tail + 1) & 0xf;
        for i in 0..min(len, 256) {
            if let Some(byte_ref) = buf_iter.next() {
                self.arr[begin + i] = unsafe { *byte_ref };
                write_size += 1;
            } else {
                return write_size;
            }
        }
        self.len_vec.push_back(write_size);
        write_size
    }
    pub fn read_datagram(&mut self, buf: UserBuffer) -> usize {
        let len = buf.len();
        let read_len = self.len_vec.pop_front().unwrap();
        let mut read_size = 0;
        let mut buf_iter = buf.into_iter();

        let begin = self.head * 256;
        self.head = (self.head + 1) & 0xf;
        for i in 0..min(len, read_len) {
            if let Some(byte_ref) = buf_iter.next() {
                unsafe {
                    *byte_ref = self.arr[begin + i];
                }
                read_size += 1;
            } else {
                return read_size;
            }
        }
        read_size
    }
    pub fn available_write(&self) -> usize {
        if self.len_vec.len() >= 16 {
            return usize::MAX;
        } else {
            return 0;
        }
    }
    pub fn available_read(&self) -> usize {
        if self.len_vec.len() == 0 {
            return usize::MAX;
        } else {
            return 0;
        }
    }
}

impl File for MailBox {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, buf: UserBuffer) -> usize {
        let mut ring_buffer = self.buffer.lock();
        let try_read = ring_buffer.available_read();
        if buf.len() == 0 || try_read != 0 {
            try_read
        } else {
            ring_buffer.read_datagram(buf)
        }
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut ring_buffer = self.buffer.lock();
        let try_write = ring_buffer.available_write();
        if buf.len() == 0 || try_write != 0 {
            try_write
        } else {
            ring_buffer.write_datagram(buf)
        }
    }
}
