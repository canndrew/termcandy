use tokio::io::{AsyncRead, AsyncWrite};
use std::io;
use std::sync::Arc;
use std::io::{Read, Write};
use futures::Async;
use mio::{Evented, Poll, Token, Ready, PollOpt};

use crate::io::OwnedEventedFd;

pub fn non_blocking_stdio() -> io::Result<(NonBlockingStdin, NonBlockingStdout)> {
    let non_blocking = NonBlocking::new()?;
    let non_blocking = Arc::new(non_blocking);
    let stdin = NonBlockingStdin {
        _non_blocking: non_blocking.clone(),
        inner: OwnedEventedFd(0),
    };
    let stdout = NonBlockingStdout {
        _non_blocking: non_blocking,
        inner: OwnedEventedFd(1),
    };
    Ok((stdin, stdout))
}

struct NonBlocking {
    was_blocking: bool,
}

impl NonBlocking {
    pub fn new() -> io::Result<NonBlocking> {
        let flags = unsafe { libc::fcntl(0, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        let was_blocking = flags & libc::O_NONBLOCK != 0;
        if !was_blocking {
            let res = unsafe { libc::fcntl(0, libc::F_SETFL, flags | libc::O_NONBLOCK) };
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(NonBlocking {
            was_blocking,
        })
    }
}

impl Drop for NonBlocking {
    fn drop(&mut self) {
        if self.was_blocking {
            unsafe {
                let flags = libc::fcntl(0, libc::F_GETFL);
                if flags >= 0 {
                    libc::fcntl(0, libc::F_SETFL, flags & !libc::O_NONBLOCK);
                }
            }
        }
    }
}

pub struct NonBlockingStdout {
    _non_blocking: Arc<NonBlocking>,
    inner: OwnedEventedFd,
}

pub struct NonBlockingStdin {
    _non_blocking: Arc<NonBlocking>,
    inner: OwnedEventedFd,
}

impl Evented for NonBlockingStdout {
    fn register(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        self.inner.register(poll, token, interest, opts)
    }

    fn reregister(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        self.inner.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.inner.deregister(poll)
    }
}

impl Evented for NonBlockingStdin {
    fn register(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        self.inner.register(poll, token, interest, opts)
    }

    fn reregister(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        self.inner.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.inner.deregister(poll)
    }
}

impl Write for NonBlockingStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Read for NonBlockingStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl AsyncWrite for NonBlockingStdout {
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        self.inner.shutdown()
    }
}

impl AsyncRead for NonBlockingStdin {}

