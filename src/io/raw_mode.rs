use termion;
use termion::raw::IntoRawMode;
use tokio::io::AsyncWrite;
use futures::Async;
use std::io::{self, Write};

pub struct RawMode<W: AsyncWrite> {
    inner: termion::raw::RawTerminal<W>,
}

impl<W: AsyncWrite> RawMode<W> {
    pub fn new(inner: W) -> io::Result<RawMode<W>> {
        let inner = inner.into_raw_mode()?;
        Ok(RawMode {
            inner: inner,
        })
    }
}

impl<W: AsyncWrite> Write for RawMode<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: AsyncWrite> AsyncWrite for RawMode<W> {
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        self.inner.shutdown()
    }
}


