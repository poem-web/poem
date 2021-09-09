use std::{io::Result, net::SocketAddr};

use tokio::{
    io::Result as IoResult,
    net::{TcpStream, ToSocketAddrs},
};

use crate::listener::{Acceptor, IntoAcceptor};

/// A TCP listener.
pub struct TcpListener<T> {
    addr: T,
}

impl<T> TcpListener<T> {
    /// Binds to the provided address, and returns a [`TcpListener<T>`].
    pub fn bind(addr: T) -> Self {
        Self { addr }
    }
}

#[async_trait::async_trait]
impl<T: ToSocketAddrs + Send> IntoAcceptor for TcpListener<T> {
    type Acceptor = TcpAcceptor;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        Ok(TcpAcceptor { listener })
    }
}

/// A acceptor that accepts TCP connections.
pub struct TcpAcceptor {
    listener: tokio::net::TcpListener,
}

#[async_trait::async_trait]
impl Acceptor for TcpAcceptor {
    type Addr = SocketAddr;
    type Io = TcpStream;

    #[inline]
    fn local_addr(&self) -> IoResult<Self::Addr> {
        self.listener.local_addr()
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, Self::Addr)> {
        self.listener.accept().await
    }
}
