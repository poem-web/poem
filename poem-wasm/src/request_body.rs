use std::{
    io::{Error as IoError, ErrorKind, Result as IoResult},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, ReadBuf};

use crate::ffi;

pub(crate) struct RequestBodyStream;

impl AsyncRead for RequestBodyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let mut data = [0u8; 4096];
        let mut ret_len = 0u32;

        let res = unsafe {
            ffi::request_read_body(
                data.as_mut_ptr() as u32,
                data.len() as u32,
                &mut ret_len as *mut u32 as u32,
            )
        };

        match res {
            -1 => Poll::Ready(Err(IoError::new(
                ErrorKind::Other,
                "failed to read request body",
            ))),
            0 => Poll::Ready(Ok(())),
            n => {
                buf.put_slice(&data[..n as usize]);
                Poll::Ready(Ok(()))
            }
        }
    }
}
