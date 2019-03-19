use std::io;
use tokio::reactor::PollEvented2;
use tokio::io::AsyncRead;
use crate::io::NonBlockingStdin;
use futures::{Async, Future, Stream};

const BUFFER_SIZE: usize = 1024;
// Total guess, but I doubt there's any longer than this.
const ESCAPE_CODE_LEN_MAX: usize = 256;

pub struct Events {
    inner: PollEvented2<NonBlockingStdin>,
    buffer: [u8; BUFFER_SIZE],
    start: usize,
    end: usize,
}

impl Events {
    pub fn new(stdin: NonBlockingStdin) -> io::Result<Events> {
        let inner = PollEvented2::new(stdin);
        Ok(Events {
            inner,
            buffer: [0u8; BUFFER_SIZE],
            start: 0,
            end: 0,
        })
    }

    pub fn key<'a>(&'a mut self, key: termion::event::Key) -> impl Future<Item = (), Error = io::Error> + 'a {
        self
        .filter_map(move |event| match event {
            termion::event::Event::Key(got) if got == key => Some(()),
            _ => None,
        })
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(opt, _)| opt.unwrap())
    }

    pub fn any_key<'a>(&'a mut self) -> impl Future<Item = termion::event::Key, Error = io::Error> + 'a {
        self
        .filter_map(move |event| match event {
            termion::event::Event::Key(got) => Some(got),
            _ => None,
        })
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(opt, _)| opt.unwrap())
    }
}

impl Stream for Events {
    type Item = termion::event::Event;
    type Error = io::Error;

    fn poll(&mut self) -> io::Result<Async<Option<termion::event::Event>>> {
        loop {
            let end = if self.end >= self.start {
                let len = self.end - self.start;
                if len >= ESCAPE_CODE_LEN_MAX {
                    break;
                }
                if self.start == 0 { BUFFER_SIZE - 1 } else { BUFFER_SIZE }
            } else {
                let len = BUFFER_SIZE + self.end - self.start;
                if len >= ESCAPE_CODE_LEN_MAX {
                    break;
                }
                self.start - 1
            };
            match self.inner.poll_read(&mut self.buffer[self.end..end])? {
                Async::Ready(n) => {
                    self.end += n;
                    if self.end == BUFFER_SIZE {
                        self.end = 0;
                    }
                }
                Async::NotReady => break,
            }
        }

        if self.start == self.end {
            return Ok(Async::NotReady);
        }

        let c = self.buffer[self.start];
        let mut iter = BufferIter {
            buffer: &mut self.buffer,
            start: (self.start + 1) % BUFFER_SIZE,
            end: self.end,
        };

        match termion::event::parse_event(c, &mut iter) {
            Ok(event) => {
                self.start = iter.start;
                Ok(Async::Ready(Some(event)))
            },
            Err(_) => Ok(Async::NotReady),
        }
    }
}

struct BufferIter<'a> {
    buffer: &'a mut [u8; BUFFER_SIZE],
    start: usize,
    end: usize,
}

impl<'a> Iterator for BufferIter<'a> {
    type Item = io::Result<u8>;

    fn next(&mut self) -> Option<io::Result<u8>> {
        if self.end == self.start {
            return Some(Err(io::ErrorKind::WouldBlock.into()));
        }

        let ret = self.buffer[self.start];
        self.start = (self.start + 1) % BUFFER_SIZE;
        Some(Ok(ret))
    }
}


