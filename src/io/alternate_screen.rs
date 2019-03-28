use std::env;
use tokio::io::{AsyncWrite, WriteAll};
use std::io::{self, Write};
use futures::{Future, Async};
use crate::io::force_write;
use mio::{Evented, Poll, Token, Ready, PollOpt};

const ENTER_ALTERNATE_SCREEN_SEQUENCE: &'static str = csi!("?1049h");

const EXIT_ALTERNATE_SCREEN_SEQUENCE: &'static str = csi!("?1049l");

pub struct AlternateScreen<W: AsyncWrite> {
    inner: W,
    enabled: bool,
}

impl<W: AsyncWrite> AlternateScreen<W> {
    pub fn new(inner: W) -> MakeAlternateScreen<W> {
        let enabled = env::var("TERMCANDY_NO_ALT_SCREEN").map(|s| s != "1").unwrap_or(true);
        MakeAlternateScreen {
            inner: if enabled {
                tokio::io::write_all(inner, ENTER_ALTERNATE_SCREEN_SEQUENCE)
            } else {
                tokio::io::write_all(inner, "")
            },
            enabled,
        }
    }
}

impl<W: AsyncWrite> Drop for AlternateScreen<W> {
    fn drop(&mut self) {
        if self.enabled {
            force_write(&mut self.inner, EXIT_ALTERNATE_SCREEN_SEQUENCE);
        }
    }
}

impl<W: AsyncWrite> Write for AlternateScreen<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: AsyncWrite> AsyncWrite for AlternateScreen<W> {
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        self.inner.shutdown()
    }
}

pub struct MakeAlternateScreen<W: AsyncWrite> {
    inner: WriteAll<W, &'static str>,
    enabled: bool,
}

impl<W: AsyncWrite> Future for MakeAlternateScreen<W> {
    type Item = AlternateScreen<W>;
    type Error = io::Error;

    fn poll(&mut self) -> io::Result<Async<AlternateScreen<W>>> {
        match self.inner.poll()? {
            Async::Ready((w, _)) => Ok(Async::Ready(AlternateScreen { inner: w, enabled: self.enabled })),
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

impl<W: AsyncWrite + Evented> Evented for AlternateScreen<W> {
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

