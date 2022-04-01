use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(crate) struct UniteStream<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> UniteStream<R, W> {
    pub(crate) fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R, W> AsyncRead for UniteStream<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}

impl<R, W> AsyncWrite for UniteStream<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let this = &mut *self;
        println!("write");
        Pin::new(&mut this.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = &mut *self;
        Pin::new(&mut this.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = &mut *self;
        Pin::new(&mut this.writer).poll_shutdown(cx)
    }
}
