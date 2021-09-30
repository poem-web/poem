//! Commonly used listeners.

mod combined;
mod tcp;
#[cfg(feature = "tls")]
mod tls;
#[cfg(unix)]
mod unix;

use std::{
    fmt::Display,
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

pub use combined::{CombinedAcceptor, CombinedListener, CombinedStream};
pub use tcp::{TcpAcceptor, TcpListener};
#[cfg(feature = "tls")]
pub use tls::{TlsAcceptor, TlsConfig, TlsListener};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result as IoResult};
#[cfg(unix)]
pub use unix::{UnixAcceptor, UnixListener};

use crate::web::RemoteAddr;

/// Represents a acceptor type.
#[async_trait::async_trait]
pub trait Acceptor: Send + Sync {
    /// Address type.
    type Addr: Send + Display + 'static;

    /// IO stream type.
    type Io: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Returns the local address that this listener is bound to.
    fn local_addr(&self) -> IoResult<Vec<Self::Addr>>;

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding IO stream and the remote peer’s
    /// address will be returned.
    async fn accept(&mut self) -> IoResult<(Self::Io, Self::Addr)>;
}

/// An owned dynamically typed Acceptor for use in cases where you can’t
/// statically type your result or need to add some indirection.
pub type BoxAcceptor = Box<dyn Acceptor<Addr = RemoteAddr, Io = BoxIo>>;

/// Extension trait for [`Acceptor`].
pub trait AcceptorExt: Acceptor {
    /// Wrap the acceptor in a Box.
    fn boxed(self) -> BoxAcceptor
    where
        Self: Sized + 'static,
    {
        Box::new(WrappedAcceptor(self))
    }
}

impl<T: Acceptor> AcceptorExt for T {}

/// Represents a listener that can be listens for incoming connections.
#[async_trait::async_trait]
pub trait Listener: Send {
    /// The acceptor type.
    type Acceptor: Acceptor;

    /// Create a acceptor instance.
    async fn into_acceptor(self) -> IoResult<Self::Acceptor>;

    /// Combine two listeners.
    ///
    /// You can call this function multiple times to combine more listeners.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::listener::{Listener, TcpListener};
    ///
    /// let listener = TcpListener::bind("0.0.0.0:80").combine(TcpListener::bind("0.0.0.0:81"));
    /// ```
    fn combine<T>(self, other: T) -> CombinedListener<Self, T>
    where
        Self: Sized,
    {
        CombinedListener::new(self, other)
    }

    /// Consume this listener and return a new TLS listener.
    #[cfg(feature = "tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
    fn tls(self, config: TlsConfig) -> TlsListener<Self>
    where
        Self: Sized,
    {
        TlsListener::new(config, self)
    }
}

/// An IO type for BoxAcceptor.
pub struct BoxIo {
    reader: Box<dyn AsyncRead + Send + Unpin + 'static>,
    writer: Box<dyn AsyncWrite + Send + Unpin + 'static>,
}

impl BoxIo {
    fn new(io: impl AsyncRead + AsyncWrite + Send + Unpin + 'static) -> Self {
        let (reader, writer) = tokio::io::split(io);
        Self {
            reader: Box::new(reader),
            writer: Box::new(writer),
        }
    }
}

impl AsyncRead for BoxIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for BoxIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let this = &mut *self;
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

struct WrappedAcceptor<T: Acceptor>(T);

#[async_trait::async_trait]
impl<T: Acceptor> Acceptor for WrappedAcceptor<T> {
    type Addr = RemoteAddr;
    type Io = BoxIo;

    fn local_addr(&self) -> IoResult<Vec<Self::Addr>> {
        self.0
            .local_addr()
            .map(|addrs| addrs.into_iter().map(RemoteAddr::new).collect())
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, Self::Addr)> {
        self.0
            .accept()
            .await
            .map(|(io, addr)| (BoxIo::new(io), RemoteAddr::new(addr)))
    }
}

#[cfg(test)]
mod tests {
    use super::{AcceptorExt, *};
    use crate::listener::TcpListener;

    #[tokio::test]
    #[should_panic]
    #[allow(unused_variables, unused_assignments)]
    async fn test_box_acceptor() {
        let mut a = TcpListener::bind("0.0.0.0:3000")
            .into_acceptor()
            .await
            .unwrap()
            .boxed();

        a = TcpListener::bind("0.0.0.0:3000")
            .tls(TlsConfig::new())
            .combine(TcpListener::bind("0.0.0.0:3001"))
            .into_acceptor()
            .await
            .unwrap()
            .boxed();
    }
}
