use std::{
    pin::Pin,
    task::{Context, Poll},
};

use poem_wasm::{ffi, Subscription};
use tokio::io::{AsyncRead, Error, ErrorKind, ReadBuf, Result};

use crate::runtime::wasi::reactor;

pub(crate) struct RequestReader;

impl AsyncRead for RequestReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        unsafe {
            let data = buf.initialize_unfilled();
            let mut bytes_read = 0u32;
            let res = ffi::read_request_body(
                data.as_mut_ptr() as u32,
                data.len() as u32,
                &mut bytes_read as *mut _ as u32,
            );

            match res {
                ffi::ERRNO_OK => {
                    buf.advance(bytes_read as usize);
                    Poll::Ready(Ok(()))
                }
                ffi::ERRNO_WOULD_BLOCK => {
                    reactor::register(Subscription::read_request_body(), cx);
                    Poll::Pending
                }
                _ => Poll::Ready(Err(Error::new(ErrorKind::Other, "unknown error"))),
            }
        }
    }
}
