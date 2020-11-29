use super::*;

use tokio::io::ReadBuf;

pub struct CycleBuffer<const CAPACITY: usize> {
    buffer: [MaybeUninit<u8>; CAPACITY],
    len: usize,
    start: usize,
}

pub struct CycleReadBuf<'a> {
    read_buf: ReadBuf<'a>,
    len: &'a mut usize,
}

impl<const CAPACITY: usize> CycleBuffer<CAPACITY> {
    pub fn new() -> CycleBuffer<CAPACITY> {
        CycleBuffer {
            buffer: MaybeUninit::uninit_array(),
            len: 0,
            start: 0,
        }
    }

    pub fn get_uninitialized<'a>(&'a mut self) -> Option<CycleReadBuf<'a>> {
        if self.len == CAPACITY {
            return None;
        }

        let end = self.start + self.len;
        let buffer = if end < CAPACITY {
            &mut self.buffer[end..CAPACITY]
        } else {
            &mut self.buffer[(end - CAPACITY)..self.start]
        };
        let read_buf = ReadBuf::uninit(buffer);
        Some(CycleReadBuf {
            read_buf,
            len: &mut self.len,
        })
    }

    pub fn get_initialized(&self) -> &[u8] {
        let end = self.start + self.len;
        let slice = if end <= CAPACITY {
            &self.buffer[self.start..end]
        } else {
            &self.buffer[self.start..CAPACITY]
        };
        unsafe {
            MaybeUninit::slice_assume_init_ref(slice)
        }
    }

    pub fn consume_initialized(&mut self, amount: usize) {
        assert!(amount <= self.len);
        self.start += amount;
        if self.start >= CAPACITY {
            self.start -= CAPACITY;
        }
        self.len -= amount;
    }

    pub fn iter_initialized<'a>(&'a mut self) -> IterInitialized<'a, CAPACITY> {
        IterInitialized {
            cycle_buffer: self,
            amount_read: 0,
        }
    }
}

impl<'a> AsMut<ReadBuf<'a>> for CycleReadBuf<'a> {
    fn as_mut(&mut self) -> &mut ReadBuf<'a> {
        &mut self.read_buf
    }
}

impl<'a> Drop for CycleReadBuf<'a> {
    fn drop(&mut self) {
        *self.len += self.read_buf.filled().len();
    }
}

pub struct IterInitialized<'a, const CAPACITY: usize> {
    cycle_buffer: &'a mut CycleBuffer<CAPACITY>,
    amount_read: usize,
}

impl<'a, const CAPACITY: usize> Iterator for IterInitialized<'a, CAPACITY> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.amount_read < self.cycle_buffer.len {
            let mut pos = self.cycle_buffer.start + self.amount_read;
            if pos >= CAPACITY {
                pos -= CAPACITY;
            }
            let byte = unsafe {
                self.cycle_buffer.buffer[pos].assume_init()
            };
            self.amount_read += 1;
            return Some(byte);
        }
        None
    }
}

impl<'a, const CAPACITY: usize> IterInitialized<'a, CAPACITY> {
    pub fn consume_read(self) {
        self.cycle_buffer.consume_initialized(self.amount_read);
    }

    pub fn take_read(self) -> Vec<u8> {
        let mut ret = Vec::with_capacity(self.amount_read);
        let mut remaining = self.amount_read;
        loop {
            let initialized = self.cycle_buffer.get_initialized();
            let initialized_len = initialized.len();
            if initialized_len < remaining {
                remaining -= initialized_len;
                ret.extend(initialized);
                self.cycle_buffer.consume_initialized(initialized_len);
                continue;
            } else {
                ret.extend(&initialized[..remaining]);
                self.cycle_buffer.consume_initialized(remaining);
                return ret;
            }
        }
    }

    pub fn read_to_completion(&self) -> bool {
        self.cycle_buffer.len == self.amount_read
    }
}

