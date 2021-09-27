//! Commonly used listeners.

mod combined;
mod tcp;
#[cfg(feature = "tls")]
mod tls;
#[cfg(unix)]
mod unix;

use std::fmt::Display;

pub use combined::{CombinedAcceptor, CombinedListener, CombinedStream};
pub use tcp::{TcpAcceptor, TcpListener};
#[cfg(feature = "tls")]
pub use tls::{TlsAcceptor, TlsConfig, TlsListener};
use tokio::io::{AsyncRead, AsyncWrite, Result as IoResult};
#[cfg(unix)]
pub use unix::{UnixAcceptor, UnixListener};

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
