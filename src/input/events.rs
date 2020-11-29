use super::*;

use crate::terminal::NonBlockingStdin;
use tokio::time::Instant;
use termion::event::{Event, Key};

use crate::cycle_buffer::CycleBuffer;

const BUFFER_SIZE: usize = 1024;

#[pin_project]
pub struct Events {
    #[pin]
    inner: NonBlockingStdin,
    cycle_buffer: CycleBuffer<BUFFER_SIZE>,
    #[pin]
    escape_timeout: Option<tokio::time::Sleep>,
}

impl Events {
    pub fn new(stdin: NonBlockingStdin) -> Events {
        Events {
            inner: stdin,
            cycle_buffer: CycleBuffer::new(),
            escape_timeout: None,
        }
    }
}

impl Stream for Events {
    type Item = io::Result<Event>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<io::Result<Event>>> {
        trace!("in Events::poll()");

        let mut this = self.project();
        loop {
            // fill our read buffer
            loop {
                if let Some(mut cycle_read_buf) = this.cycle_buffer.get_uninitialized() {
                    let read_buf = cycle_read_buf.as_mut();

                    match this.inner.as_mut().poll_read(cx, read_buf) {
                        Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                        Poll::Ready(Ok(())) => (),
                        Poll::Pending => break,
                    }
                }
            }

            let mut iter = this.cycle_buffer.iter_initialized();
            let c = match iter.next() {
                Some(c) => c,
                None => return Poll::Pending,
            };
            match termion::event::parse_event(c, &mut (&mut iter).map(Ok)) {
                Ok(event) => {
                    iter.consume_read();
                    return Poll::Ready(Some(Ok(event)));
                },
                Err(_) => {
                    // Termion failed to parse an event from the input. Either the input buffer
                    // doesn't contain a complete event yet, or it contains unparseable garbage.

                    if iter.read_to_completion() {
                        // Termion reached the end of the buffer while parsing. There may not be
                        // enough input yet to read an event.

                        if c == 0x1b {
                            // The buffer contains either an incomplete escape sequence, or the
                            // user pressed the escape button. The only way to tell is based on
                            // timings. If a complete escape sequence doesn't arrive within a
                            // timeout then we treat it as an escape key-press.

                            let timer_running = match this.escape_timeout.as_mut().as_pin_mut() {
                                None => {
                                    // We don't yet have a timer running to decide how to treat the
                                    // escape character. Start one now and restart the function so
                                    // that we read any new input and poll the timer.
                                    false
                                },

                                Some(escape_timeout) => {
                                    // We already have a timeout runing.

                                    match escape_timeout.poll(cx) {
                                        Poll::Ready(()) => {
                                            // The timeout has expired. Drop the escape char from
                                            // the buffer and treat it as an escape key-press.
                                            true
                                        },
                                        Poll::Pending => {
                                            // The timer hasn't expired yet. Wait a little longer.
                                            return Poll::Pending;
                                        },
                                    }
                                },
                            };
                            if timer_running {
                                this.escape_timeout.set(None);
                                this.cycle_buffer.consume_initialized(1);
                                return Poll::Ready(Some(Ok(Event::Key(Key::Esc))));
                            } else {
                                let deadline = Instant::now() + Duration::from_millis(200);
                                this.escape_timeout.set(Some(tokio::time::sleep_until(deadline)));
                                continue;
                            }
                        } else {
                            // The buffer contains something else unparseable. We probably need to
                            // wait for more input to arrive.

                            return Poll::Pending;
                        }
                    } else {
                        // Termion gave up parsing the buffer before it got to the end. Must be
                        // something bad in it. Take all the bytes that termion read and emit them
                        // as an unsupported event.

                        let bytes = iter.take_read();
                        let event = Event::Unsupported(bytes);
                        return Poll::Ready(Some(Ok(event)));
                    }
                },
            }
        }
    }
}

