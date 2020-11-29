use super::*;

use crate::terminal::Blocking;

/// A sequence of escape codes to enable terminal mouse support.
const ENTER_MOUSE_SEQUENCE: &'static [u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h";

/// A sequence of escape codes to disable terminal mouse support.
const EXIT_MOUSE_SEQUENCE: &'static [u8] = b"\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l";

#[pin_project]
pub struct MouseTerminal<W: Write> {
    #[pin]
    inner: W,
    drop_written: bool,
}

impl<W: Write + AsyncWrite> MouseTerminal<W> {
    pub async fn new(mut inner: W) -> io::Result<MouseTerminal<W>> {
        inner.write_all(ENTER_MOUSE_SEQUENCE)?;
        Ok(MouseTerminal { inner, drop_written: false })
    }
}

impl<W: Write> Drop for MouseTerminal<W> {
    fn drop(&mut self) {
        if let Ok(blocking) = Blocking::new() {
            let _ = Write::write_all(&mut self.inner, EXIT_MOUSE_SEQUENCE);
            drop(blocking);
        }
    }
}

impl<W: Write> Write for MouseTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Write + AsyncWrite> AsyncWrite for MouseTerminal<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        this.inner.poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        this.inner.poll_shutdown(cx)
    }
}

