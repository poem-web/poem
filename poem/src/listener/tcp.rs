use std::io::Result;

use http::uri::Scheme;
use tokio::{
    io::Result as IoResult,
    net::{TcpListener as TokioTcpListener, TcpStream, ToSocketAddrs},
};

use crate::{
    listener::{Acceptor, Listener},
    web::{LocalAddr, RemoteAddr},
};

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
impl<T: ToSocketAddrs + Send> Listener for TcpListener<T> {
    type Acceptor = TcpAcceptor;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let listener = TokioTcpListener::bind(self.addr).await?;
        let local_addr = listener.local_addr().map(|addr| LocalAddr(addr.into()))?;
        Ok(TcpAcceptor {
            local_addr,
            listener,
        })
    }
}

/// A acceptor that accepts TCP connections.
pub struct TcpAcceptor {
    local_addr: LocalAddr,
    listener: TokioTcpListener,
}

impl TcpAcceptor {
    /// Creates new `TcpAcceptor` from a `std::net::TcpListener`.
    pub fn from_std(listener: std::net::TcpListener) -> Result<Self> {
        let local_addr = listener.local_addr().map(|addr| LocalAddr(addr.into()))?;
        Ok(Self {
            local_addr,
            listener: TokioTcpListener::from_std(listener)?,
        })
    }

    /// Creates new `TcpAcceptor` from a `tokio::net::TcpListener`.
    pub fn from_tokio(listener: tokio::net::TcpListener) -> Result<Self> {
        let local_addr = listener.local_addr().map(|addr| LocalAddr(addr.into()))?;
        Ok(Self {
            local_addr,
            listener,
        })
    }
}

#[async_trait::async_trait]
impl Acceptor for TcpAcceptor {
    type Io = TcpStream;

    #[inline]
    fn local_addr(&self) -> Vec<LocalAddr> {
        vec![self.local_addr.clone()]
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        self.listener.accept().await.map(|(io, addr)| {
            (
                io,
                self.local_addr.clone(),
                RemoteAddr(addr.into()),
                Scheme::HTTP,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    #[tokio::test]
    async fn tcp_listener() {
        let listener = TcpListener::bind("127.0.0.1:0");
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().remove(0);

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
