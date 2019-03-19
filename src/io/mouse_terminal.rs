use tokio::io::AsyncWrite;
use tokio::io::{WriteAll};
use std::time::{Duration, Instant};
use std::thread;
use std::io::{self, Write};
use futures::{Future, Async};

/// A sequence of escape codes to enable terminal mouse support.
const ENTER_MOUSE_SEQUENCE: &'static str = csi!("?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h");

/// A sequence of escape codes to disable terminal mouse support.
const EXIT_MOUSE_SEQUENCE: &'static str = csi!("?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l");

pub struct MouseTerminal<W: AsyncWrite> {
    inner: W,
}

impl<W: AsyncWrite> MouseTerminal<W> {
    pub fn new(inner: W) -> MakeMouseTerminal<W> {
        MakeMouseTerminal {
            inner: tokio::io::write_all(inner, ENTER_MOUSE_SEQUENCE),
        }
    }
}

impl<W: AsyncWrite> Drop for MouseTerminal<W> {
    fn drop(&mut self) {
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut write_all = tokio::io::write_all(&mut self.inner, EXIT_MOUSE_SEQUENCE);
        loop {
            if let Ok(Async::NotReady) = write_all.poll() {
                if deadline < Instant::now() {
                    thread::yield_now();
                    continue;
                }
            }
            break;
        }
    }
}

impl<W: AsyncWrite> Write for MouseTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: AsyncWrite> AsyncWrite for MouseTerminal<W> {
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        self.inner.shutdown()
    }
}

pub struct MakeMouseTerminal<W: AsyncWrite> {
    inner: WriteAll<W, &'static str>,
}

impl<W: AsyncWrite> Future for MakeMouseTerminal<W> {
    type Item = MouseTerminal<W>;
    type Error = io::Error;

    fn poll(&mut self) -> io::Result<Async<MouseTerminal<W>>> {
        match self.inner.poll()? {
            Async::Ready((w, _)) => Ok(Async::Ready(MouseTerminal { inner: w })),
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}


