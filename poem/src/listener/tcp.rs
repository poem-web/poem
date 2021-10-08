use std::io::Result;

use tokio::{
    io::Result as IoResult,
    net::{TcpListener as TokioTcpListener, TcpStream, ToSocketAddrs},
};

use crate::{
    listener::{Acceptor, Listener},
    web::RemoteAddr,
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
        Ok(TcpAcceptor { listener })
    }
}

/// A acceptor that accepts TCP connections.
pub struct TcpAcceptor {
    listener: TokioTcpListener,
}

#[async_trait::async_trait]
impl Acceptor for TcpAcceptor {
    type Io = TcpStream;

    #[inline]
    fn local_addr(&self) -> IoResult<Vec<RemoteAddr>> {
        Ok(vec![self.listener.local_addr()?.into()])
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, RemoteAddr)> {
        self.listener
            .accept()
            .await
            .map(|(io, addr)| (io, addr.into()))
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
        let local_addr = acceptor.local_addr().unwrap().remove(0);

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
