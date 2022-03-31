use std::{
    pin::Pin,
    task::{Context, Poll},
};

use poem_wasm::{ffi, Subscription};
use tokio::io::{AsyncWrite, Error, ErrorKind};

use crate::runtime::wasi::reactor;

pub(crate) struct ResponseWriter;

impl AsyncWrite for ResponseWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        unsafe {
            let mut bytes_written = 0u32;
            let res = ffi::write_response_body(
                buf.as_ptr() as u32,
                buf.len() as u32,
                &mut bytes_written as *mut _ as u32,
            );

            match res {
                ffi::ERRNO_OK => Poll::Ready(Ok(bytes_written as usize)),
                ffi::ERRNO_WOULD_BLOCK => {
                    reactor::register(Subscription::write_response_body(), cx);
                    Poll::Pending
                }
                _ => Poll::Ready(Err(Error::new(ErrorKind::Other, "unknown error"))),
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
