use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf, Result};
use poem_wasm::ffi;

pub struct RequestReader;

impl AsyncRead for RequestReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        unsafe {buf.unfilled_mut().}
        ffi::read_request_body()
        todo!()
    }
}
