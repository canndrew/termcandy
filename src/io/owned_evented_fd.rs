use libc;
use std::io;
use tokio::io::{AsyncWrite, AsyncRead};
use std::io::{Read, Write};
use futures::Async;
use mio::{Evented, Poll, Token, Ready, PollOpt};
use mio::unix::EventedFd;
use std::os::unix::io::RawFd;

pub struct OwnedEventedFd(pub RawFd);

impl Evented for OwnedEventedFd {
    fn register(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        let evented_fd = EventedFd(&self.0);
        evented_fd.register(poll, token, interest, opts)
    }

    fn reregister(
        &self, 
        poll: &Poll, 
        token: Token, 
        interest: Ready, 
        opts: PollOpt
    ) -> io::Result<()> {
        let evented_fd = EventedFd(&self.0);
        evented_fd.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        let evented_fd = EventedFd(&self.0);
        evented_fd.deregister(poll)
    }
}

impl Write for OwnedEventedFd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let res = libc::write(self.0, buf.as_ptr() as *const libc::c_void, buf.len());
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(res as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        unsafe {
            let res = libc::fsync(self.0);
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

impl Read for OwnedEventedFd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let res = libc::read(self.0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(res as usize)
        }
    }
}

impl AsyncWrite for OwnedEventedFd {
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        unsafe {
            let res = libc::shutdown(self.0, libc::SHUT_WR);
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Async::Ready(()))
        }
    }
}

impl AsyncRead for OwnedEventedFd {}

