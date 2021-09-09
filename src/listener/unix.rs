use std::{io::Result, path::Path};

use tokio::{
    io::Result as IoResult,
    net::{unix::SocketAddr, UnixListener, UnixStream},
};

use crate::listener::{Acceptor, IntoAcceptor};

/// A Unix domain socket listener.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixListener<T> {
    path: T,
}

impl<T> UnixListener<T> {
    /// Binds to the provided address, and returns a [`UnixListener<T>`].
    pub fn bind(addr: impl AsRef<T>) -> Self {
        Self { path }
    }
}

#[async_trait::async_trait]
impl<T: AsRef<Path> + Send> IntoAcceptor for UnixListener<T> {
    type Acceptor = UnixAcceptor;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        Ok(UnixAcceptor { listener })
    }
}

/// A acceptor that accepts connections.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixAcceptor {
    listener: tokio::net::UnixListener,
}

#[async_trait::async_trait]
impl Acceptor for UnixAcceptor {
    type Addr = SocketAddr;
    type Io = UnixStream;

    #[inline]
    fn local_addr(&self) -> IoResult<Self::Addr> {
        self.listener.local_addr()
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, Self::Addr)> {
        self.listener.accept().await
    }
}
