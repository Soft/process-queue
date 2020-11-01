use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::{Context, Poll};

use log::debug;
use tokio::io::unix;
use tokio::io::{AsyncRead, ReadBuf};

use crate::ffi;

pub struct Fd(Option<RawFd>);

impl Fd {
    pub fn new(fd: RawFd) -> Self {
        Self(Some(fd))
    }
}

impl Read for Fd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        ffi::read(self.as_raw_fd(), unsafe {
            &mut *(buf as *mut _ as *mut [MaybeUninit<u8>])
        })
    }
}

impl Write for Fd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        ffi::write(self.as_raw_fd(), buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl From<Fd> for RawFd {
    fn from(mut fd: Fd) -> Self {
        fd.0.take().unwrap()
    }
}

impl AsRawFd for Fd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.unwrap()
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        if let Some(fd) = self.0 {
            debug!("closing fd {}", fd);
            let _ = ffi::close(fd);
        }
    }
}

pub struct AsyncFd(unix::AsyncFd<Fd>);

impl AsyncFd {
    pub fn from_blocking(fd: Fd) -> io::Result<Self> {
        let flags = ffi::getfl(fd.as_raw_fd())?;
        ffi::setfl(fd.as_raw_fd(), flags | libc::O_NONBLOCK)?;
        Ok(Self(unix::AsyncFd::new(fd)?))
    }
}

impl AsRawFd for AsyncFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsyncRead for AsyncFd {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut ready = match self.0.poll_read_ready(cx) {
                Poll::Ready(Ok(ready)) => ready,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => return Poll::Pending,
            };

            let buffer = unsafe { buf.unfilled_mut() };
            match ffi::read(self.0.as_raw_fd(), buffer) {
                Ok(bytes) => {
                    unsafe { buf.assume_init(bytes) };
                    buf.advance(bytes);
                    return Poll::Ready(Ok(()));
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    ready.clear_ready();
                }
                Err(err) => {
                    return Poll::Ready(Err(err));
                }
            }
        }
    }
}
