use std::{
    fmt::{self, Display, Formatter},
    io::Result,
    path::Path,
};

use thiserror::private::PathAsDisplay;
use tokio::{
    io::Result as IoResult,
    net::{unix::SocketAddr, UnixListener as TokioUnixListener, UnixStream},
};

use crate::listener::{Acceptor, Listener};

/// A Unix domain socket listener.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixListener<T> {
    path: T,
}

impl<T> UnixListener<T> {
    /// Binds to the provided address, and returns a [`UnixListener<T>`].
    pub fn bind(path: T) -> Self {
        Self { path }
    }
}

#[async_trait::async_trait]
impl<T: AsRef<Path> + Send> Listener for UnixListener<T> {
    type Acceptor = UnixAcceptor;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let listener = TokioUnixListener::bind(self.path)?;
        Ok(UnixAcceptor { listener })
    }
}

/// Unix domain socket address.
#[derive(Debug)]
pub struct UnixSocketAddr(SocketAddr);

impl Display for UnixSocketAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0.as_pathname() {
            Some(path) => write!(f, "{}", path.as_display()),
            None => write!(f, "unnamed"),
        }
    }
}

/// A acceptor that accepts connections.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixAcceptor {
    listener: TokioUnixListener,
}

#[async_trait::async_trait]
impl Acceptor for UnixAcceptor {
    type Addr = UnixSocketAddr;
    type Io = UnixStream;

    #[inline]
    fn local_addr(&self) -> IoResult<Vec<Self::Addr>> {
        self.listener
            .local_addr()
            .map(|addr| vec![UnixSocketAddr(addr)])
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, Self::Addr)> {
        let (stream, addr) = self.listener.accept().await?;
        Ok((stream, UnixSocketAddr(addr)))
    }
}
