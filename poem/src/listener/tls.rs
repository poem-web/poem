use std::sync::Arc;

use futures_util::{stream::BoxStream, Stream, StreamExt};
use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_rustls::{
    rustls::{
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth,
        RootCertStore, ServerConfig,
    },
    server::TlsStream,
};
use tokio_util::either::Either;

use crate::{
    listener::{Acceptor, Listener},
    web::{LocalAddr, RemoteAddr},
};

#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
enum TlsClientAuth {
    Off,
    Optional(Vec<u8>),
    Required(Vec<u8>),
}

/// TLS Config.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsConfig {
    cert: Vec<u8>,
    key: Vec<u8>,
    client_auth: TlsClientAuth,
    ocsp_resp: Vec<u8>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TlsConfig {
    /// Create a new tls config object.
    pub fn new() -> Self {
        Self {
            cert: Vec::new(),
            key: Vec::new(),
            client_auth: TlsClientAuth::Off,
            ocsp_resp: Vec::new(),
        }
    }

    /// Sets the certificates.
    pub fn cert(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.cert = cert.into();
        self
    }

    /// Sets the private key.
    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = key.into();
        self
    }

    /// Sets the trust anchor for optional client authentication.
    pub fn client_auth_optional(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Optional(trust_anchor.into());
        self
    }

    /// Sets the trust anchor for required client authentication.
    pub fn client_auth_required(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Required(trust_anchor.into());
        self
    }

    /// Sets the DER-encoded OCSP response.
    pub fn ocsp_resp(mut self, ocsp_resp: impl Into<Vec<u8>>) -> Self {
        self.ocsp_resp = ocsp_resp.into();
        self
    }

    pub(crate) fn create_server_config(&self) -> IoResult<ServerConfig> {
        let cert = tokio_rustls::rustls::internal::pemfile::certs(&mut self.cert.as_slice())
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls certificates"))?;
        let key = {
            let mut pkcs8 = tokio_rustls::rustls::internal::pemfile::pkcs8_private_keys(
                &mut self.key.as_slice(),
            )
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls private keys"))?;
            if !pkcs8.is_empty() {
                pkcs8.remove(0)
            } else {
                let mut rsa = tokio_rustls::rustls::internal::pemfile::rsa_private_keys(
                    &mut self.key.as_slice(),
                )
                .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls private keys"))?;

                if !rsa.is_empty() {
                    rsa.remove(0)
                } else {
                    return Err(IoError::new(
                        ErrorKind::Other,
                        "failed to parse tls private keys",
                    ));
                }
            }
        };

        fn read_trust_anchor(mut trust_anchor: &[u8]) -> IoResult<RootCertStore> {
            let mut store = RootCertStore::empty();
            if let Ok((0, _)) | Err(()) = store.add_pem_file(&mut trust_anchor) {
                Err(IoError::new(
                    ErrorKind::Other,
                    "failed to parse tls trust anchor",
                ))
            } else {
                Ok(store)
            }
        }

        let client_auth = match &self.client_auth {
            TlsClientAuth::Off => NoClientAuth::new(),
            TlsClientAuth::Optional(trust_anchor) => {
                AllowAnyAnonymousOrAuthenticatedClient::new(read_trust_anchor(trust_anchor)?)
            }
            TlsClientAuth::Required(trust_anchor) => {
                AllowAnyAuthenticatedClient::new(read_trust_anchor(trust_anchor)?)
            }
        };

        let mut server_config = ServerConfig::new(client_auth);
        server_config
            .set_single_cert_with_ocsp_and_sct(cert, key, self.ocsp_resp.clone(), Vec::new())
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        server_config.set_protocols(&["h2".into(), "http/1.1".into()]);

        Ok(server_config)
    }
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol.
///
/// NOTE: You cannot create it directly and should use the
/// [`tls`](crate::listener::Listener::tls) method to create it, because it
/// needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsListener<T, S> {
    inner: T,
    config_stream: S,
}

impl<T, S> TlsListener<T, S>
where
    T: Listener,
    S: IntoTlsConfigStream,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        Self {
            inner,
            config_stream,
        }
    }
}

#[async_trait::async_trait]
impl<T: Listener, S: IntoTlsConfigStream> Listener for TlsListener<T, S> {
    type Acceptor = TlsAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(TlsAcceptor::new(
            self.inner.into_acceptor().await?,
            Box::pin(self.config_stream.into_stream()?),
        ))
    }
}

/// Represents a type that can convert into tls config stream.
pub trait IntoTlsConfigStream: Send + 'static {
    /// Represents a tls config stream.
    type Stream: Stream<Item = TlsConfig> + Send + 'static;

    /// Consume itself and return tls config stream.
    fn into_stream(self) -> IoResult<Self::Stream>;
}

impl<T> IntoTlsConfigStream for T
where
    T: Stream<Item = TlsConfig> + Send + 'static,
{
    type Stream = Self;

    fn into_stream(self) -> IoResult<Self::Stream> {
        Ok(self)
    }
}

impl IntoTlsConfigStream for TlsConfig {
    type Stream = futures_util::stream::Once<futures_util::future::Ready<TlsConfig>>;

    fn into_stream(self) -> IoResult<Self::Stream> {
        let _ = self.create_server_config()?;
        Ok(futures_util::stream::once(futures_util::future::ready(
            self,
        )))
    }
}

/// A TLS or SSL protocol acceptor.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsAcceptor<T> {
    inner: T,
    config_stream: Option<BoxStream<'static, TlsConfig>>,
    current_tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
}

impl<T> TlsAcceptor<T> {
    pub(crate) fn new<S>(inner: T, config_stream: S) -> Self
    where
        S: Stream<Item = TlsConfig> + Send + 'static,
    {
        TlsAcceptor {
            inner,
            config_stream: Some(config_stream.boxed()),
            current_tls_acceptor: None,
        }
    }
}

#[async_trait::async_trait]
impl<T: Acceptor> Acceptor for TlsAcceptor<T> {
    type Io = TlsStream<T::Io>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
        loop {
            let mut config_stream = match &mut self.config_stream {
                Some(config_stream) => Either::Left(config_stream),
                None => Either::Right(futures_util::stream::pending()),
            };

            tokio::select! {
                res = config_stream.next() => {
                    if let Some(tls_config) = res {
                        match tls_config.create_server_config() {
                            Ok(server_config) => {
                                if self.current_tls_acceptor.is_some() {
                                    tracing::info!("tls config changed.");
                                } else {
                                    tracing::info!("tls config loaded.");
                                }
                                self.current_tls_acceptor = Some(tokio_rustls::TlsAcceptor::from(Arc::new(server_config)));

                            },
                            Err(err) => tracing::error!(error = %err, "invalid tls config."),
                        }
                    } else {
                        self.config_stream = None;
                    }
                }
                res = self.inner.accept() => {
                    let (stream, local_addr, remote_addr) = res?;
                    let tls_acceptor = match &self.current_tls_acceptor {
                        Some(tls_acceptor) => tls_acceptor,
                        None => return Err(IoError::new(ErrorKind::Other, "no valid tls config.")),
                    };
                    let stream = tls_acceptor.accept(stream).await?;
                    return Ok((stream, local_addr, remote_addr));
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
        time::Duration,
    };
    use tokio_rustls::rustls::ClientConfig;

    use super::*;
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn tls_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").tls(
            TlsConfig::new()
                .cert(include_bytes!("certs/cert1.pem").as_ref())
                .key(include_bytes!("certs/key1.pem").as_ref()),
        );
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().pop().unwrap();

        tokio::spawn(async move {
            let mut config = ClientConfig::new();
            config
                .root_store
                .add_pem_file(&mut include_bytes!("certs/chain1.pem").as_ref())
                .unwrap();

            let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
            let domain = webpki::DNSNameRef::try_from_ascii_str("testserver.com").unwrap();
            let stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            let mut stream = connector.connect(domain, stream).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }

    #[tokio::test]
    async fn tls_hot_loading() {
        let tls_config = async_stream::stream! {
            yield TlsConfig::new()
                .cert(include_bytes!("certs/cert1.pem").as_ref())
                .key(include_bytes!("certs/key1.pem").as_ref());

            tokio::time::sleep(Duration::from_secs(1)).await;

            yield TlsConfig::new()
                .cert(include_bytes!("certs/cert2.pem").as_ref())
                .key(include_bytes!("certs/key2.pem").as_ref());

            tokio::time::sleep(Duration::from_secs(1)).await;

            yield TlsConfig::new()
                .cert(include_bytes!("certs/cert1.pem").as_ref())
                .key(include_bytes!("certs/key1.pem").as_ref());
        };

        let listener = TcpListener::bind("127.0.0.1:0").tls(tls_config);
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().pop().unwrap();

        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _, _)) = acceptor.accept().await {
                    assert_eq!(stream.read_i32().await.unwrap(), 10);
                }
            }
        });

        async fn do_request(
            local_addr: &LocalAddr,
            domain: &str,
            chain: Option<&[u8]>,
            success: bool,
        ) {
            let mut config = ClientConfig::new();

            if let Some(mut chain) = chain {
                config.root_store.add_pem_file(&mut chain).ok();
            }

            let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
            let domain = webpki::DNSNameRef::try_from_ascii_str(domain).unwrap();
            let stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();

            match connector.connect(domain, stream).await {
                Ok(mut stream) => {
                    if !success {
                        panic!();
                    }
                    stream.write_i32(10).await.unwrap();
                }
                Err(err) => {
                    if success {
                        panic!("{}", err);
                    }
                }
            }
        }

        do_request(
            &local_addr,
            "testserver.com",
            Some(include_bytes!("certs/chain1.pem").as_ref()),
            true,
        )
        .await;

        tokio::time::sleep(Duration::from_secs(1)).await;

        do_request(&local_addr, "example.com", None, false).await;

        tokio::time::sleep(Duration::from_secs(1)).await;

        do_request(
            &local_addr,
            "testserver.com",
            Some(include_bytes!("certs/chain1.pem").as_ref()),
            true,
        )
        .await;
    }
}
