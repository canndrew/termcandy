use super::*;

use tokio::io::ReadBuf;

fn set_stdio_non_blocking(non_blocking: bool) -> io::Result<bool> {
    let flags = unsafe {
        libc::fcntl(0, libc::F_GETFL)
    };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    let was_non_blocking = flags & libc::O_NONBLOCK != 0;
    if was_non_blocking != non_blocking {
        let new_flags = if non_blocking {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };
        let res = unsafe {
            libc::fcntl(0, libc::F_SETFL, new_flags)
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(was_non_blocking)
}

fn handle_os_error<T>(res: isize, func: impl FnOnce(usize) -> T) -> Poll<io::Result<T>> {
    if res < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            Poll::Pending
        } else {
            Poll::Ready(Err(err))
        }
    } else {
        Poll::Ready(Ok(func(res as usize)))
    }
}

struct NonBlockingStdinInner {
    _priv: (),
}

struct NonBlockingStdoutInner {
    _priv: (),
}

impl AsRawFd for NonBlockingStdinInner {
    fn as_raw_fd(&self) -> RawFd {
        0
    }
}

impl AsRawFd for NonBlockingStdoutInner {
    fn as_raw_fd(&self) -> RawFd {
        1
    }
}

pub fn non_blocking_stdio() -> io::Result<(NonBlockingStdin, NonBlockingStdout)> {
    let non_blocking = NonBlocking::new()?;
    let non_blocking = Arc::new(non_blocking);
    let stdin = NonBlockingStdin {
        _non_blocking: non_blocking.clone(),
        inner: AsyncFd::new(NonBlockingStdinInner { _priv: () })?,
    };
    let stdout = NonBlockingStdout {
        _non_blocking: non_blocking,
        inner: AsyncFd::new(NonBlockingStdoutInner { _priv: () })?,
    };
    Ok((stdin, stdout))
}

struct NonBlocking {
    was_non_blocking: bool,
}

impl NonBlocking {
    pub fn new() -> io::Result<NonBlocking> {
        let was_non_blocking = !set_stdio_non_blocking(true)?;
        Ok(NonBlocking { was_non_blocking })
    }
}

impl Drop for NonBlocking {
    fn drop(&mut self) {
        if !self.was_non_blocking {
            let _ = set_stdio_non_blocking(false);
        }
    }
}

#[pin_project]
pub struct NonBlockingStdout {
    _non_blocking: Arc<NonBlocking>,
    #[pin]
    inner: AsyncFd<NonBlockingStdoutInner>,
}

#[pin_project]
pub struct NonBlockingStdin {
    _non_blocking: Arc<NonBlocking>,
    #[pin]
    inner: AsyncFd<NonBlockingStdinInner>,
}

impl Write for NonBlockingStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = unsafe {
            libc::write(1, buf.as_ptr() as *mut _, buf.len())
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(res as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        let res = unsafe {
            libc::fsync(1)
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Read for NonBlockingStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let res = unsafe {
            libc::read(0, buf.as_mut_ptr() as *mut _, buf.len())
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(res as usize)
    }
}

impl AsyncWrite for NonBlockingStdout {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        match this.inner.poll_write_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Ready(Ok(mut ready)) => {
                ready.with_poll(move || {
                    let res = unsafe {
                        libc::write(1, buf.as_ptr() as *mut _, buf.len())
                    };
                    if res < 0 {
                        let err = io::Error::last_os_error();
                        if err.kind() == io::ErrorKind::WouldBlock {
                            return Poll::Pending;
                        }
                        return Poll::Ready(Err(err));
                    }
                    Poll::Ready(Ok(res as usize))
                })
            },
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        let res = unsafe {
            libc::fsync(1)
        };
        if res < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Poll::Pending;
            }
            return Poll::Ready(Err(err));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        let res = unsafe {
            libc::shutdown(1, libc::SHUT_WR)
        };
        if res < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                Poll::Pending
            } else {
                Poll::Ready(Err(err))
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncRead for NonBlockingStdin {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        read_buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.inner.poll_read_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Ready(Ok(mut ready)) => {
                ready.with_poll(move || {
                    let buffer = unsafe {
                        read_buf.unfilled_mut()
                    };
                    let res = unsafe {
                        libc::read(0, buffer.as_mut_ptr() as *mut _, buffer.len())
                    };
                    handle_os_error(res, move |res| {
                        unsafe {
                            read_buf.assume_init(res);
                        }
                        read_buf.advance(res);
                    })
                })
            },
        }

    }
}

pub struct Blocking {
    was_non_blocking: bool,
}

impl Blocking {
    pub fn new() -> io::Result<Blocking> {
        let was_non_blocking = set_stdio_non_blocking(false)?;
        Ok(Blocking { was_non_blocking })
    }
}

impl Drop for Blocking {
    fn drop(&mut self) {
        if self.was_non_blocking {
            let _ = set_stdio_non_blocking(true);
        }
    }
}

