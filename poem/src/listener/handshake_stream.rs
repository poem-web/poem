use std::{
    future::Future,
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{future::BoxFuture, FutureExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};

enum State<S> {
    Handshaking(BoxFuture<'static, Result<S>>),
    Ready(S),
}

/// A handshake stream for tls.
pub struct HandshakeStream<S> {
    state: State<S>,
}

impl<S> HandshakeStream<S> {
    pub(crate) fn new<F>(handshake: F) -> Self
    where
        F: Future<Output = Result<S>> + Send + 'static,
    {
        Self {
            state: State::Handshaking(handshake.boxed()),
        }
    }
}

impl<S> AsyncRead for HandshakeStream<S>
where
    S: AsyncRead + Unpin + Send + 'static,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        let this = &mut *self;

        loop {
            match &mut this.state {
                State::Handshaking(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(s)) => this.state = State::Ready(s),
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                },
                State::Ready(stream) => return Pin::new(stream).poll_read(cx, buf),
            }
        }
    }
}

impl<S> AsyncWrite for HandshakeStream<S>
where
    S: AsyncWrite + Unpin + Send + 'static,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, Error>> {
        let this = &mut *self;

        loop {
            match &mut this.state {
                State::Handshaking(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(s)) => this.state = State::Ready(s),
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                },
                State::Ready(stream) => return Pin::new(stream).poll_write(cx, buf),
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        let this = &mut *self;

        loop {
            match &mut this.state {
                State::Handshaking(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(s)) => this.state = State::Ready(s),
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                },
                State::Ready(stream) => return Pin::new(stream).poll_flush(cx),
            }
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        let this = &mut *self;

        loop {
            match &mut this.state {
                State::Handshaking(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(s)) => this.state = State::Ready(s),
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => return Poll::Pending,
                },
                State::Ready(stream) => return Pin::new(stream).poll_shutdown(cx),
            }
        }
    }
}
