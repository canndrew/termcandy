use super::*;

use crate::terminal::Blocking;

const ENTER_ALTERNATE_SCREEN_SEQUENCE: &'static [u8] = b"\x1b[?1049h";

const EXIT_ALTERNATE_SCREEN_SEQUENCE: &'static [u8] = b"\x1b[?1049l";

#[pin_project]
pub struct AlternateScreen<W: Write> {
    #[pin]
    inner: W,
    enabled: bool,
}

impl<W: Write + AsyncWrite> AlternateScreen<W> {
    pub async fn new(mut inner: W) -> io::Result<AlternateScreen<W>> {
        let enabled = env::var("TERMCANDY_NO_ALT_SCREEN").map(|s| s != "1").unwrap_or(true);
        if enabled {
            inner.write_all(ENTER_ALTERNATE_SCREEN_SEQUENCE)?;
        }
        Ok(AlternateScreen {
            inner,
            enabled,
        })
    }
}

impl<W: Write> Drop for AlternateScreen<W> {
    fn drop(&mut self) {
        if let Ok(blocking) = Blocking::new() {
            let _ = Write::write_all(&mut self.inner, EXIT_ALTERNATE_SCREEN_SEQUENCE);
            drop(blocking);
        }
    }
}

impl<W: Write> Write for AlternateScreen<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Write + AsyncWrite> AsyncWrite for AlternateScreen<W> {
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

