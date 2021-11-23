//! Commonly used listeners.

mod combined;
#[cfg(feature = "native-tls")]
mod native_tls;
#[cfg(feature = "rustls")]
mod rustls;
mod tcp;
#[cfg(any(feature = "rustls", feature = "native-tls"))]
mod tls;
#[cfg(unix)]
mod unix;

use std::{
    convert::Infallible,
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

pub use combined::{Combined, CombinedStream};
#[cfg(feature = "native-tls")]
pub use native_tls::{NativeTlsAcceptor, NativeTlsConfig, NativeTlsListener};
#[cfg(feature = "rustls")]
pub use rustls::{RustlsAcceptor, RustlsConfig, RustlsListener};
pub use tcp::{TcpAcceptor, TcpListener};
#[cfg(any(feature = "rustls", feature = "native-tls"))]
pub use tls::IntoTlsConfigStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result as IoResult};
#[cfg(unix)]
pub use unix::{UnixAcceptor, UnixListener};

use crate::web::{LocalAddr, RemoteAddr};

/// Represents a acceptor type.
#[async_trait::async_trait]
pub trait Acceptor: Send {
    /// IO stream type.
    type Io: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Returns the local address that this listener is bound to.
    fn local_addr(&self) -> Vec<LocalAddr>;

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding IO stream and the remote peer’s
    /// address will be returned.
    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)>;
}

/// An owned dynamically typed Acceptor for use in cases where you can’t
/// statically type your result or need to add some indirection.
pub type BoxAcceptor = Box<dyn Acceptor<Io = BoxIo>>;

/// Extension trait for [`Acceptor`].
pub trait AcceptorExt: Acceptor {
    /// Combine two acceptors.
    #[must_use]
    fn combine<T>(self, other: T) -> Combined<Self, T>
    where
        Self: Sized,
    {
        Combined::new(self, other)
    }

    /// Wrap the acceptor in a Box.
    fn boxed(self) -> BoxAcceptor
    where
        Self: Sized + 'static,
    {
        Box::new(WrappedAcceptor(self))
    }

    /// Consume this acceptor and return a new TLS acceptor with [`rustls`](https://crates.io/crates/rustls).
    #[cfg(feature = "rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    fn rustls<S>(self, config_stream: S) -> RustlsAcceptor<Self, S>
    where
        Self: Sized,
        S: futures_util::Stream<Item = RustlsConfig> + Send + Unpin + 'static,
    {
        RustlsAcceptor::new(self, config_stream)
    }

    /// Consume this acceptor and return a new TLS acceptor with [`native-tls`](https://crates.io/crates/native-tls).
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    fn native_tls<S>(self, config_stream: S) -> NativeTlsAcceptor<Self, S>
    where
        Self: Sized,
        S: futures_util::Stream<Item = NativeTlsConfig> + Send + Unpin + 'static,
    {
        NativeTlsAcceptor::new(self, config_stream)
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
    /// let listener = TcpListener::bind("127.0.0.1:80").combine(TcpListener::bind("127.0.0.1:81"));
    /// ```
    #[must_use]
    fn combine<T>(self, other: T) -> Combined<Self, T>
    where
        Self: Sized,
    {
        Combined::new(self, other)
    }

    /// Consume this listener and return a new TLS listener with [`rustls`](https://crates.io/crates/rustls).
    #[cfg(feature = "rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[must_use]
    fn rustls<S: IntoTlsConfigStream<RustlsConfig>>(
        self,
        config_stream: S,
    ) -> RustlsListener<Self, S>
    where
        Self: Sized,
    {
        RustlsListener::new(self, config_stream)
    }

    /// Consume this listener and return a new TLS listener with [`native-tls`](https://crates.io/crates/native-tls).
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[must_use]
    fn native_tls<S: IntoTlsConfigStream<NativeTlsConfig>>(
        self,
        config_stream: S,
    ) -> NativeTlsListener<Self, S>
    where
        Self: Sized,
    {
        NativeTlsListener::new(self, config_stream)
    }
}

#[async_trait::async_trait]
impl Listener for Infallible {
    type Acceptor = Infallible;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        unreachable!()
    }
}

#[async_trait::async_trait]
impl<T: Acceptor + ?Sized> Acceptor for Box<T> {
    type Io = T::Io;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.as_ref().local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
        self.as_mut().accept().await
    }
}

#[async_trait::async_trait]
impl Acceptor for Infallible {
    type Io = BoxIo;

    fn local_addr(&self) -> Vec<LocalAddr> {
        vec![]
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
        unreachable!()
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
    type Io = BoxIo;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.0.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
        self.0
            .accept()
            .await
            .map(|(io, local_addr, remote_addr)| (BoxIo::new(io), local_addr, remote_addr))
    }
}

#[cfg(test)]
mod tests {
    use super::{AcceptorExt, *};
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn combined_listener() {
        let a = TcpListener::bind("127.0.0.1:0");
        let b = TcpListener::bind("127.0.0.1:0");
        let _ = a.combine(b);
    }

    #[tokio::test]
    async fn combined_acceptor() {
        let a = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();

        let b = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();

        let _ = a.combine(b);
    }
}
