use std::{
    io::{ErrorKind, Result as IoResult},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

use crate::wasi::reactor::register_write;

pub struct FdWriter {
    fd: libwasi::Fd,
}

impl FdWriter {
    #[inline]
    pub fn new(fd: libwasi::Fd) -> Self {
        FdWriter { fd }
    }
}

impl AsyncWrite for FdWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        let this = &mut *self;

        return match write(this.fd, buf) {
            Ok(sz) => Poll::Ready(Ok(sz)),
            Err(err) if err == libwasi::ERRNO_AGAIN => {
                register_write(this.fd, cx);
                Poll::Pending
            }
            Err(err) => Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, err.to_string()))),
        };
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }
}

fn write(fd: libwasi::Fd, buf: &[u8]) -> Result<usize, libwasi::Errno> {
    unsafe {
        libwasi::fd_write(
            fd,
            &[libwasi::Ciovec {
                buf: buf.as_ptr(),
                buf_len: buf.len(),
            }],
        )
    }
}
