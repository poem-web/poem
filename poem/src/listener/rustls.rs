use std::{collections::HashMap, sync::Arc};

use futures_util::{
    stream::{BoxStream, Chain, Pending},
    Stream, StreamExt,
};
use http::uri::Scheme;
use rustls_pemfile::Item;
use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_rustls::{
    rustls::{
        server::{
            AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, ClientHello,
            NoClientAuth, ResolvesServerCert,
        },
        sign::{self, CertifiedKey},
        Certificate, PrivateKey, RootCertStore, ServerConfig,
    },
    server::TlsStream,
};

use crate::{
    listener::{Acceptor, HandshakeStream, IntoTlsConfigStream, Listener},
    web::{LocalAddr, RemoteAddr},
};

#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
enum TlsClientAuth {
    Off,
    Optional(Vec<u8>),
    Required(Vec<u8>),
}

/// Rustls certificate
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
#[derive(Default)]
pub struct RustlsCertificate {
    cert: Vec<u8>,
    key: Vec<u8>,
    ocsp_resp: Vec<u8>,
}

impl RustlsCertificate {
    /// Create a [`RustlsCertificate`] object.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the certificates.
    #[must_use]
    pub fn cert(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.cert = cert.into();
        self
    }

    /// Sets the private key.
    #[must_use]
    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = key.into();
        self
    }

    /// Sets the DER-encoded OCSP response.
    #[must_use]
    pub fn ocsp_resp(mut self, ocsp_resp: impl Into<Vec<u8>>) -> Self {
        self.ocsp_resp = ocsp_resp.into();
        self
    }
}

impl RustlsCertificate {
    fn create_certificate_key(&self) -> IoResult<CertifiedKey> {
        let cert = rustls_pemfile::certs(&mut self.cert.as_slice())
            .map(|mut certs| certs.drain(..).map(Certificate).collect())
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls certificates"))?;

        let priv_key = {
            loop {
                let key = match rustls_pemfile::read_one(&mut self.key.as_slice())? {
                    Some(Item::RSAKey(key)) => key,
                    Some(Item::PKCS8Key(key)) => key,
                    Some(Item::ECKey(key)) => key,
                    None => {
                        return Err(IoError::new(
                            ErrorKind::Other,
                            "failed to parse tls private keys",
                        ))
                    }
                    _ => continue,
                };
                if !key.is_empty() {
                    break PrivateKey(key);
                }
            }
        };

        let key = sign::any_supported_type(&priv_key)
            .map_err(|_| IoError::new(ErrorKind::Other, "invalid private key"))?;

        Ok(CertifiedKey {
            cert,
            key,
            ocsp: if !self.ocsp_resp.is_empty() {
                Some(self.ocsp_resp.clone())
            } else {
                None
            },
            sct_list: None,
        })
    }
}

/// Rustls Config.
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
pub struct RustlsConfig {
    certificates: HashMap<String, RustlsCertificate>,
    fallback: Option<RustlsCertificate>,
    client_auth: TlsClientAuth,
}

impl Default for RustlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RustlsConfig {
    /// Create a new tls config object.
    pub fn new() -> Self {
        Self {
            certificates: HashMap::new(),
            fallback: Default::default(),
            client_auth: TlsClientAuth::Off,
        }
    }

    /// Sets the certificates.
    #[deprecated = "replaced by `RustlsConfig::fallback`"]
    #[must_use]
    pub fn cert(mut self, cert: impl Into<Vec<u8>>) -> Self {
        match &mut self.fallback {
            Some(fallback) => fallback.cert = cert.into(),
            None => {
                self.fallback = Some(RustlsCertificate {
                    cert: cert.into(),
                    ..Default::default()
                })
            }
        }
        self
    }

    /// Sets the private key.
    #[deprecated = "replaced by `RustlsConfig::fallback`"]
    #[must_use]
    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        match &mut self.fallback {
            Some(fallback) => fallback.key = key.into(),
            None => {
                self.fallback = Some(RustlsCertificate {
                    key: key.into(),
                    ..Default::default()
                })
            }
        }
        self
    }

    /// Sets the DER-encoded OCSP response.
    #[deprecated = "replaced by `RustlsConfig::fallback`"]
    #[must_use]
    pub fn ocsp_resp(mut self, ocsp_resp: impl Into<Vec<u8>>) -> Self {
        match &mut self.fallback {
            Some(fallback) => fallback.ocsp_resp = ocsp_resp.into(),
            None => {
                self.fallback = Some(RustlsCertificate {
                    ocsp_resp: ocsp_resp.into(),
                    ..Default::default()
                })
            }
        }
        self
    }

    /// If the certificate corresponding to the SNI name is not found, it will
    /// fall back to this certificate.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use poem::listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener};
    ///
    /// # let cert_bytes: Vec<u8> = todo!();
    /// # let key_bytes: Vec<u8> = todo!();
    ///
    /// let config =
    ///     RustlsConfig::new().fallback(RustlsCertificate::new().cert(cert_bytes).key(key_bytes));
    /// let listener = TcpListener::bind("127.0.0.1:3000").rustls(config);
    /// ```
    pub fn fallback(mut self, certificate: RustlsCertificate) -> Self {
        self.fallback = Some(certificate);
        self
    }

    /// Add a new certificate to be used for the given SNI `name`.
    pub fn certificate(mut self, name: impl Into<String>, certificate: RustlsCertificate) -> Self {
        self.certificates.insert(name.into(), certificate);
        self
    }

    /// Sets the trust anchor for optional client authentication.
    #[must_use]
    pub fn client_auth_optional(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Optional(trust_anchor.into());
        self
    }

    /// Sets the trust anchor for required client authentication.
    #[must_use]
    pub fn client_auth_required(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Required(trust_anchor.into());
        self
    }

    fn create_server_config(&self) -> IoResult<ServerConfig> {
        let fallback = self
            .fallback
            .as_ref()
            .map(|fallback| fallback.create_certificate_key())
            .transpose()?
            .map(Arc::new);
        let mut certifcate_keys = HashMap::new();

        for (name, certificate) in &self.certificates {
            certifcate_keys.insert(
                name.clone(),
                Arc::new(certificate.create_certificate_key()?),
            );
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

        let mut server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(client_auth)
            .with_cert_resolver(Arc::new(ResolveServerCert {
                certifcate_keys,
                fallback,
            }));
        server_config.alpn_protocols = vec!["h2".into(), "http/1.1".into()];

        Ok(server_config)
    }
}

fn read_trust_anchor(mut trust_anchor: &[u8]) -> IoResult<RootCertStore> {
    let mut store = RootCertStore::empty();
    let ders = rustls_pemfile::certs(&mut trust_anchor)?;
    for der in ders {
        store
            .add(&Certificate(der))
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
    }
    Ok(store)
}

impl<T> IntoTlsConfigStream<RustlsConfig> for T
where
    T: Stream<Item = RustlsConfig> + Send + 'static,
{
    type Stream = Self;

    fn into_stream(self) -> IoResult<Self::Stream> {
        Ok(self)
    }
}

impl IntoTlsConfigStream<RustlsConfig> for RustlsConfig {
    type Stream = futures_util::stream::Once<futures_util::future::Ready<RustlsConfig>>;

    fn into_stream(self) -> IoResult<Self::Stream> {
        let _ = self.create_server_config()?;
        Ok(futures_util::stream::once(futures_util::future::ready(
            self,
        )))
    }
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol with [`rustls`](https://crates.io/crates/rustls).
///
/// NOTE: You cannot create it directly and should use the
/// [`rustls`](crate::listener::Listener::rustls) method to create it, because
/// it needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
pub struct RustlsListener<T, S> {
    inner: T,
    config_stream: S,
}

impl<T, S> RustlsListener<T, S>
where
    T: Listener,
    S: IntoTlsConfigStream<RustlsConfig>,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        Self {
            inner,
            config_stream,
        }
    }
}

#[async_trait::async_trait]
impl<T: Listener, S: IntoTlsConfigStream<RustlsConfig>> Listener for RustlsListener<T, S> {
    type Acceptor = RustlsAcceptor<T::Acceptor, BoxStream<'static, RustlsConfig>>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(RustlsAcceptor::new(
            self.inner.into_acceptor().await?,
            self.config_stream.into_stream()?.boxed(),
        ))
    }
}

/// A TLS or SSL protocol acceptor with [`rustls`](https://crates.io/crates/rustls).
#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
pub struct RustlsAcceptor<T, S> {
    inner: T,
    config_stream: Chain<S, Pending<RustlsConfig>>,
    current_tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
}

impl<T, S> RustlsAcceptor<T, S>
where
    S: Stream<Item = RustlsConfig> + Send + Unpin + 'static,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        RustlsAcceptor {
            inner,
            config_stream: config_stream.chain(futures_util::stream::pending()),
            current_tls_acceptor: None,
        }
    }
}

#[async_trait::async_trait]
impl<T, S> Acceptor for RustlsAcceptor<T, S>
where
    S: Stream<Item = RustlsConfig> + Send + Unpin + 'static,
    T: Acceptor,
{
    type Io = HandshakeStream<TlsStream<T::Io>>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        loop {
            tokio::select! {
                res = self.config_stream.next() => {
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
                        unreachable!()
                    }
                }
                res = self.inner.accept() => {
                    let (stream, local_addr, remote_addr, _) = res?;
                    let tls_acceptor = match &self.current_tls_acceptor {
                        Some(tls_acceptor) => tls_acceptor,
                        None => return Err(IoError::new(ErrorKind::Other, "no valid tls config.")),
                    };

                    let stream = HandshakeStream::new(tls_acceptor.accept(stream));
                    return Ok((stream, local_addr, remote_addr, Scheme::HTTPS));
                }
            }
        }
    }
}

struct ResolveServerCert {
    certifcate_keys: HashMap<String, Arc<CertifiedKey>>,
    fallback: Option<Arc<CertifiedKey>>,
}

impl ResolvesServerCert for ResolveServerCert {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        client_hello
            .server_name()
            .and_then(|name| self.certifcate_keys.get(name).map(Arc::clone))
            .or_else(|| self.fallback.clone())
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };
    use tokio_rustls::rustls::{ClientConfig, ServerName};

    use super::*;
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn tls_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").rustls(
            RustlsConfig::new().fallback(
                RustlsCertificate::new()
                    .cert(include_bytes!("certs/cert1.pem").as_ref())
                    .key(include_bytes!("certs/key1.pem").as_ref()),
            ),
        );
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().pop().unwrap();

        tokio::spawn(async move {
            let config = ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(
                    read_trust_anchor(include_bytes!("certs/chain1.pem")).unwrap(),
                )
                .with_no_client_auth();

            let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
            let domain = ServerName::try_from("testserver.com").unwrap();
            let stream = TcpStream::connect(*local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            let mut stream = connector.connect(domain, stream).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
