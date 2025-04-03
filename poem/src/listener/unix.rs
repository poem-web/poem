use std::{
    fs::{Permissions, set_permissions},
    io::Result,
    path::Path,
};

use http::uri::Scheme;
use nix::unistd::{Gid, Uid, chown};
use tokio::{
    io::Result as IoResult,
    net::{UnixListener as TokioUnixListener, UnixStream},
};

use crate::{
    listener::{Acceptor, Listener},
    web::{LocalAddr, RemoteAddr},
};

/// A Unix domain socket listener.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixListener<T> {
    path: T,
    permissions: Option<Permissions>,
    owner: Option<(Option<Uid>, Option<Gid>)>,
}

impl<T> UnixListener<T> {
    /// Binds to the provided address, and returns a [`UnixListener<T>`].
    pub fn bind(path: T) -> Self {
        Self {
            path,
            permissions: None,
            owner: None,
        }
    }

    /// Provides permissions to be set on actual bind
    pub fn with_permissions(self, permissions: Permissions) -> Self {
        Self {
            permissions: Some(permissions),
            ..self
        }
    }

    #[cfg(unix)]
    /// Provides owner to be set on actual bind
    pub fn with_owner(self, uid: Option<u32>, gid: Option<u32>) -> Self {
        Self {
            owner: Some((uid.map(Uid::from_raw), gid.map(Gid::from_raw))),
            ..self
        }
    }
}

impl<T: AsRef<Path> + Send + Clone> Listener for UnixListener<T> {
    type Acceptor = UnixAcceptor;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let listener = match (self.permissions, self.owner) {
            (Some(permissions), Some((uid, gid))) => {
                let listener = TokioUnixListener::bind(self.path.clone())?;
                set_permissions(self.path.clone(), permissions)?;
                chown(self.path.as_ref().as_os_str(), uid, gid)?;
                listener
            }
            (Some(permissions), None) => {
                let listener = TokioUnixListener::bind(self.path.clone())?;
                set_permissions(self.path.clone(), permissions)?;
                listener
            }
            (None, Some((uid, gid))) => {
                let listener = TokioUnixListener::bind(self.path.clone())?;
                chown(self.path.as_ref().as_os_str(), uid, gid)?;
                listener
            }
            (None, None) => TokioUnixListener::bind(self.path)?,
        };

        let local_addr = listener.local_addr().map(|addr| LocalAddr(addr.into()))?;
        Ok(UnixAcceptor {
            local_addr,
            listener,
        })
    }
}

/// A acceptor that accepts connections.
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub struct UnixAcceptor {
    local_addr: LocalAddr,
    listener: TokioUnixListener,
}

impl UnixAcceptor {
    /// Creates new `UnixAcceptor` from a `std::os::unix::net::UnixListener`.
    pub fn from_std(listener: std::os::unix::net::UnixListener) -> Result<Self> {
        let listener = TokioUnixListener::from_std(listener)?;
        let local_addr = listener.local_addr().map(|addr| LocalAddr(addr.into()))?;
        Ok(Self {
            local_addr,
            listener,
        })
    }
}

impl Acceptor for UnixAcceptor {
    type Io = UnixStream;

    #[inline]
    fn local_addr(&self) -> Vec<LocalAddr> {
        vec![self.local_addr.clone()]
    }

    #[inline]
    async fn accept(&mut self) -> Result<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        let (stream, addr) = self.listener.accept().await?;
        Ok((
            stream,
            self.local_addr.clone(),
            RemoteAddr(addr.into()),
            Scheme::HTTP,
        ))
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        time::Duration,
    };

    use super::*;

    #[tokio::test]
    async fn unix_listener() {
        let listener = UnixListener::bind("test-socket");
        let mut acceptor = listener.into_acceptor().await.unwrap();

        tokio::spawn(async move {
            let mut stream = UnixStream::connect("test-socket").await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);

        tokio::time::sleep(Duration::from_secs(1)).await;
        drop(acceptor);
        std::fs::remove_file("test-socket").unwrap();
    }
}
