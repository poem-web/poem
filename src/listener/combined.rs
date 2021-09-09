use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result as IoResult};

use crate::{
    listener::{Acceptor, IntoAcceptor},
    web::RemoteAddr,
};

/// Listener for the [`combine`](crate::IntoListener::combine) method.
pub struct CombinedListener<A, B> {
    a: A,
    b: B,
}

impl<A, B> CombinedListener<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self {
        CombinedListener { a, b }
    }
}

#[async_trait::async_trait]
impl<A: IntoAcceptor, B: IntoAcceptor> IntoAcceptor for CombinedListener<A, B> {
    type Acceptor = CombinedAcceptor<A::Acceptor, B::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(CombinedAcceptor {
            a: self.a.into_acceptor().await?,
            b: self.b.into_acceptor().await?,
        })
    }
}

/// Used to combine two listeners.
pub struct CombinedAcceptor<A, B> {
    a: A,
    b: B,
}

#[async_trait::async_trait]
impl<A: Acceptor, B: Acceptor> Acceptor for CombinedAcceptor<A, B> {
    type Addr = RemoteAddr;
    type Io = CombinedStream<A::Io, B::Io>;

    fn local_addr(&self) -> IoResult<Self::Addr> {
        Ok(RemoteAddr::new(format!(
            "{}; {}",
            self.a.local_addr()?,
            self.b.local_addr()?
        )))
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, Self::Addr)> {
        tokio::select! {
            res = self.a.accept() => {
                let (stream, addr) = res?;
                Ok((CombinedStream::A(stream), RemoteAddr::new(addr)))
            }
            res = self.b.accept() => {
                let (stream, addr) = res?;
                Ok((CombinedStream::B(stream), RemoteAddr::new(addr)))
            }
        }
    }
}

/// A IO stream for CombinedAcceptor.
pub enum CombinedStream<A, B> {
    #[allow(missing_docs)]
    A(A),
    #[allow(missing_docs)]
    B(B),
}

impl<A, B> AsyncRead for CombinedStream<A, B>
where
    A: AsyncRead + Send + Unpin + 'static,
    B: AsyncRead + Send + Unpin + 'static,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let this = &mut *self;
        match this {
            CombinedStream::A(a) => Pin::new(a).poll_read(cx, buf),
            CombinedStream::B(b) => Pin::new(b).poll_read(cx, buf),
        }
    }
}

impl<A, B> AsyncWrite for CombinedStream<A, B>
where
    A: AsyncWrite + Send + Unpin + 'static,
    B: AsyncWrite + Send + Unpin + 'static,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        let this = &mut *self;
        match this {
            CombinedStream::A(a) => Pin::new(a).poll_write(cx, buf),
            CombinedStream::B(b) => Pin::new(b).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let this = &mut *self;
        match this {
            CombinedStream::A(a) => Pin::new(a).poll_flush(cx),
            CombinedStream::B(b) => Pin::new(b).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let this = &mut *self;
        match this {
            CombinedStream::A(a) => Pin::new(a).poll_shutdown(cx),
            CombinedStream::B(b) => Pin::new(b).poll_shutdown(cx),
        }
    }
}
