use std::io;
use tokio::reactor::PollEvented2;
use tokio::io::AsyncRead;
use tokio::timer::Delay;
use crate::io::NonBlockingStdin;
use futures::{Async, Future, Stream};
use log::trace;
use std::time::{Duration, Instant};
use termion::event::{Event, Key};

const BUFFER_SIZE: usize = 1024;
// Total guess, but I doubt there's any longer than this.
const ESCAPE_CODE_LEN_MAX: usize = 256;

pub struct Events {
    inner: PollEvented2<NonBlockingStdin>,
    buffer: [u8; BUFFER_SIZE],
    start: usize,
    end: usize,
    escape_timeout: Option<Delay>,
}

impl Events {
    pub fn new(stdin: NonBlockingStdin) -> io::Result<Events> {
        let inner = PollEvented2::new(stdin);
        Ok(Events {
            inner,
            buffer: [0u8; BUFFER_SIZE],
            start: 0,
            end: 0,
            escape_timeout: None,
        })
    }

    /*
    pub fn key<'a>(&'a mut self, key: Key) -> impl Future<Item = (), Error = failure::Error> + 'a {
        self
        .filter_map(move |event| match event {
            Event::Key(got) if got == key => Some(()),
            _ => None,
        })
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(opt, _)| opt.unwrap())
    }

    pub fn any_key<'a>(&'a mut self) -> impl Future<Item = Key, Error = failure::Error> + 'a {
        self
        .filter_map(move |event| match event {
            Event::Key(got) => Some(got),
            _ => None,
        })
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(opt, _)| opt.unwrap())
    }

    pub fn next<'a>(&'a mut self) -> impl Future<Item = Event, Error = failure::Error> + 'a {
        self
        .into_future()
        .map_err(|(e, _)| e)
        .map(|(opt, _)| opt.unwrap())
    }
    */
}

impl Stream for Events {
    type Item = Event;
    type Error = failure::Error;

    fn poll(&mut self) -> Result<Async<Option<Event>>, failure::Error> {
        trace!("in Events::poll()");

        loop {
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

            trace!("we've got some input!");
            let c = self.buffer[self.start];
            let mut iter = BufferIter {
                buffer: &mut self.buffer,
                start: (self.start + 1) % BUFFER_SIZE,
                end: self.end,
                reached_end: false,
            };

            trace!("parsing...");
            return match termion::event::parse_event(c, &mut iter) {
                Ok(event) => {
                    trace!("got event: {:?}", event);
                    self.start = iter.start;
                    Ok(Async::Ready(Some(event)))
                },
                Err(e) => {
                    trace!("got an error: {:?}", e);
                    if iter.reached_end {
                        trace!("we reached the end though");
                        if c == 27 {
                            trace!("but it's an escape sequence in the buffer");
                            match self.escape_timeout.take() {
                                Some(mut escape_timeout) => {
                                    trace!("and we're waiting to see if we get more input after it");
                                    match escape_timeout.poll() {
                                        Err(e) => {
                                            let e = {
                                                failure::Error::from_boxed_compat(Box::new(e))
                                                .context(format!("tokio timer died"))
                                            };
                                            Err(e.into())
                                        },
                                        Ok(Async::Ready(())) => {
                                            trace!("and we've waiting long enough. output it as an escape keystroke");
                                            self.start = iter.start;
                                            Ok(Async::Ready(Some(Event::Key(Key::Esc))))
                                        },
                                        Ok(Async::NotReady) => {
                                            trace!("but we should wait a bit longer");
                                            self.escape_timeout = Some(escape_timeout);
                                            Ok(Async::NotReady)
                                        },
                                    }
                                }
                                None => {
                                    trace!("let's wait to see if more input immediately follows");
                                    let deadline = Instant::now() + Duration::from_millis(100);
                                    self.escape_timeout = Some(Delay::new(deadline));
                                    continue;
                                },
                            }
                        } else {
                            Ok(Async::NotReady)
                        }
                    } else {
                        trace!("lets advance the buffer");
                        let bytes = if iter.start > self.start {
                            iter.buffer[self.start..iter.start].to_owned()
                        } else {
                            let mut bytes = iter.buffer[self.start..].to_owned();
                            bytes.extend(&iter.buffer[..iter.start]);
                            bytes
                        };
                        let event = Event::Unsupported(bytes);
                        self.start = iter.start;
                        Ok(Async::Ready(Some(event)))
                    }
                },
            }
        }
    }
}

struct BufferIter<'a> {
    buffer: &'a mut [u8; BUFFER_SIZE],
    start: usize,
    end: usize,
    reached_end: bool,
}

impl<'a> Iterator for BufferIter<'a> {
    type Item = io::Result<u8>;

    fn next(&mut self) -> Option<io::Result<u8>> {
        if self.end == self.start {
            self.reached_end = true;
            return Some(Err(io::ErrorKind::WouldBlock.into()));
        }

        let ret = self.buffer[self.start];
        self.start = (self.start + 1) % BUFFER_SIZE;
        Some(Ok(ret))
    }
}


