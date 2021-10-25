use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result as IoResult};

use crate::{
    listener::{Acceptor, Listener},
    web::{LocalAddr, RemoteAddr},
};

/// Listener for the [`Listener::combine`](crate::listener::Listener::combine)
/// and [`AcceptorExt::combine`](crate::listener::AcceptorExt::combine) method.
pub struct Combined<A, B> {
    a: A,
    b: B,
}

impl<A, B> Combined<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self {
        Combined { a, b }
    }
}

#[async_trait::async_trait]
impl<A: Listener, B: Listener> Listener for Combined<A, B> {
    type Acceptor = Combined<A::Acceptor, B::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(Combined {
            a: self.a.into_acceptor().await?,
            b: self.b.into_acceptor().await?,
        })
    }
}

#[async_trait::async_trait]
impl<A: Acceptor, B: Acceptor> Acceptor for Combined<A, B> {
    type Io = CombinedStream<A::Io, B::Io>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.a
            .local_addr()
            .into_iter()
            .chain(self.b.local_addr().into_iter())
            .collect()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
        tokio::select! {
            res = self.a.accept() => {
                let (stream, local_addr, remote_addr) = res?;
                Ok((CombinedStream::A(stream), local_addr, remote_addr))
            }
            res = self.b.accept() => {
                let (stream, local_addr, remote_addr) = res?;
                Ok((CombinedStream::B(stream), local_addr, remote_addr))
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

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use super::*;
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn combined() {
        let listener =
            TcpListener::bind("127.0.0.1:3001").combine(TcpListener::bind("127.0.0.1:3002"));
        let mut acceptor = listener.into_acceptor().await.unwrap();

        tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:3001").await.unwrap();
            stream.write_i32(10).await.unwrap();

            let mut stream = TcpStream::connect("127.0.0.1:3002").await.unwrap();
            stream.write_i32(20).await.unwrap();
        });

        let (mut stream, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);

        let (mut stream, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 20);
    }
}
