use std::{io::Result, net::SocketAddr};

use tokio::{
    io::Result as IoResult,
    net::{TcpListener as TokioTcpListener, TcpStream, ToSocketAddrs},
};

use crate::listener::{Acceptor, Listener};

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
        Ok(TcpAcceptor { listener })
    }
}

/// A acceptor that accepts TCP connections.
pub struct TcpAcceptor {
    listener: TokioTcpListener,
}

#[async_trait::async_trait]
impl Acceptor for TcpAcceptor {
    type Addr = SocketAddr;
    type Io = TcpStream;

    #[inline]
    fn local_addr(&self) -> IoResult<Vec<Self::Addr>> {
        Ok(vec![self.listener.local_addr()?])
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, Self::Addr)> {
        self.listener.accept().await
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
        let addr = acceptor.local_addr().unwrap().remove(0);

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
