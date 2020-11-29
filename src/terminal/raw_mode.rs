use super::*;

#[pin_project]
pub struct RawMode<W: Write + AsyncWrite> {
    #[pin]
    inner: W,
    prev_termios: libc::termios,
}

impl<W: Write + AsyncWrite> RawMode<W> {
    pub fn new(inner: W) -> io::Result<RawMode<W>> {
        let prev_termios = get_terminal_attr()?;
        let mut termios = prev_termios;
        raw_terminal_attr(&mut termios);
        set_terminal_attr(&termios)?;

        Ok(RawMode {
            inner: inner,
            prev_termios,
        })
    }
}

impl<W: Write + AsyncWrite> Drop for RawMode<W> {
    fn drop(&mut self) {
        let _ = set_terminal_attr(&self.prev_termios);
    }
}

impl<W: Write + AsyncWrite> Write for RawMode<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Write + AsyncWrite> AsyncWrite for RawMode<W> {
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

pub fn get_terminal_attr() -> io::Result<libc::termios> {
    unsafe {
        let mut termios = mem::zeroed();
        let res = libc::tcgetattr(1, &mut termios);
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(termios)
    }
}

pub fn set_terminal_attr(termios: &libc::termios) -> io::Result<()> {
    let res = unsafe {
        libc::tcsetattr(1, 0, termios)
    };
    if res < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn raw_terminal_attr(termios: &mut libc::termios) {
    unsafe {
        libc::cfmakeraw(termios)
    }
}
