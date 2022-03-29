use std::{
    io::{ErrorKind, Result as IoResult},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, ReadBuf};

use crate::wasi::reactor::register_read;

pub struct FdReader {
    fd: libwasi::Fd,
}

impl FdReader {
    #[inline]
    pub fn new(fd: libwasi::Fd) -> Self {
        FdReader { fd }
    }
}

impl AsyncRead for FdReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let this = &mut *self;
        let mut temp_buf = [0u8; 4096];

        return match read(this.fd, &mut temp_buf) {
            Ok(sz) => {
                buf.put_slice(&temp_buf[..sz]);
                Poll::Ready(Ok(()))
            }
            Err(err) if err == libwasi::ERRNO_AGAIN => {
                register_read(this.fd, cx);
                Poll::Pending
            }
            Err(err) => Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, err.to_string()))),
        };
    }
}

fn read(fd: libwasi::Fd, buf: &mut [u8]) -> Result<usize, libwasi::Errno> {
    unsafe {
        libwasi::fd_read(
            fd,
            &[libwasi::Iovec {
                buf: buf.as_mut_ptr(),
                buf_len: buf.len(),
            }],
        )
    }
}
