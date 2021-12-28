use futures_util::{
    stream::{BoxStream, Chain, Pending},
    Stream, StreamExt,
};
use http::uri::Scheme;
use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_native_tls::{native_tls::Identity, TlsStream};

use crate::{
    listener::{Acceptor, IntoTlsConfigStream, Listener},
    web::{LocalAddr, RemoteAddr},
};

/// Native TLS Config.
#[derive()]
#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
pub struct NativeTlsConfig {
    pkcs12: Vec<u8>,
    password: String,
}

impl Default for NativeTlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeTlsConfig {
    /// Create a new tls config object.
    pub fn new() -> Self {
        NativeTlsConfig {
            pkcs12: Vec::new(),
            password: String::new(),
        }
    }

    /// Sets a DER-formatted PKCS #12 archive.
    pub fn pkcs12(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.pkcs12 = data.into();
        self
    }

    /// Sets password to decrypt the key.
    pub fn password(mut self, passwd: impl Into<String>) -> Self {
        self.password = passwd.into();
        self
    }

    fn create_acceptor(&self) -> IoResult<tokio_native_tls::native_tls::TlsAcceptor> {
        let identity = Identity::from_pkcs12(&self.pkcs12, &self.password)
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        tokio_native_tls::native_tls::TlsAcceptor::new(identity)
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))
    }
}

impl<T> IntoTlsConfigStream<NativeTlsConfig> for T
where
    T: Stream<Item = NativeTlsConfig> + Send + 'static,
{
    type Stream = Self;

    fn into_stream(self) -> IoResult<Self::Stream> {
        Ok(self)
    }
}

impl IntoTlsConfigStream<NativeTlsConfig> for NativeTlsConfig {
    type Stream = futures_util::stream::Once<futures_util::future::Ready<NativeTlsConfig>>;

    fn into_stream(self) -> IoResult<Self::Stream> {
        let _ = Identity::from_pkcs12(&self.pkcs12, &self.password)
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        Ok(futures_util::stream::once(futures_util::future::ready(
            self,
        )))
    }
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol with [`native-tls`](https://crates.io/crates/native-tls).
///
/// NOTE: You cannot create it directly and should use the
/// [`native_tls`](crate::listener::Listener::native_tls) method to create it,
/// because it needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
pub struct NativeTlsListener<T, S> {
    inner: T,
    config_stream: S,
}

impl<T, S> NativeTlsListener<T, S>
where
    T: Listener,
    S: IntoTlsConfigStream<NativeTlsConfig>,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        Self {
            inner,
            config_stream,
        }
    }
}

#[async_trait::async_trait]
impl<T: Listener, S: IntoTlsConfigStream<NativeTlsConfig>> Listener for NativeTlsListener<T, S> {
    type Acceptor = NativeTlsAcceptor<T::Acceptor, BoxStream<'static, NativeTlsConfig>>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(NativeTlsAcceptor::new(
            self.inner.into_acceptor().await?,
            self.config_stream.into_stream()?.boxed(),
        ))
    }
}

/// A TLS or SSL protocol acceptor with [`native_tls`](https://crates.io/crates/native_tls).
#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
pub struct NativeTlsAcceptor<T, S> {
    inner: T,
    config_stream: Chain<S, Pending<NativeTlsConfig>>,
    current_tls_acceptor: Option<tokio_native_tls::TlsAcceptor>,
}

impl<T, S> NativeTlsAcceptor<T, S>
where
    S: Stream<Item = NativeTlsConfig> + Send + Unpin + 'static,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        NativeTlsAcceptor {
            inner,
            config_stream: config_stream.chain(futures_util::stream::pending()),
            current_tls_acceptor: None,
        }
    }
}

#[async_trait::async_trait]
impl<T, S> Acceptor for NativeTlsAcceptor<T, S>
where
    S: Stream<Item = NativeTlsConfig> + Send + Unpin + 'static,
    T: Acceptor,
{
    type Io = TlsStream<T::Io>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        loop {
            tokio::select! {
                res = self.config_stream.next() => {
                    if let Some(tls_config) = res {
                        match tls_config.create_acceptor() {
                            Ok(acceptor) => {
                                if self.current_tls_acceptor.is_some() {
                                    tracing::info!("tls config changed.");
                                } else {
                                    tracing::info!("tls config loaded.");
                                }
                                self.current_tls_acceptor = Some(tokio_native_tls::TlsAcceptor::from(acceptor));
                            },
                            Err(err) => tracing::error!(error = %err, "invalid tls config."),
                        }
                    } else {
                        unreachable!()
                    }
                }
                res = self.inner.accept() => {
                    let (stream, local_addr, remote_addr, _) = res?;
                    let tls_acceptor = match &self.current_tls_acceptor {
                        Some(tls_acceptor) => tls_acceptor,
                        None => return Err(IoError::new(ErrorKind::Other, "no valid tls config.")),
                    };
                    let stream = tls_acceptor.accept(stream).await.map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
                    return Ok((stream, local_addr, remote_addr, Scheme::HTTPS));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use super::*;
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn tls_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").native_tls(
            NativeTlsConfig::new()
                .pkcs12(include_bytes!("certs/identity.p12").as_ref())
                .password("mypass"),
        );
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().pop().unwrap();

        tokio::spawn(async move {
            let connector = tokio_native_tls::TlsConnector::from(
                tokio_native_tls::native_tls::TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap(),
            );
            let stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            let mut stream = connector.connect("127.0.0.1", stream).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
