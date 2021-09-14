use std::sync::Arc;

use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_rustls::{
    rustls::{
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth,
        RootCertStore, ServerConfig,
    },
    server::TlsStream,
};

use crate::listener::{Acceptor, IntoAcceptor};

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
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol.
///
/// NOTE: You cannot create it directly and should use the
/// [`tls`](crate::listener::IntoAcceptor::tls) method to create it, because it
/// needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsListener<T> {
    config: TlsConfig,
    inner: T,
}

impl<T: IntoAcceptor> TlsListener<T> {
    pub(crate) fn new(config: TlsConfig, inner: T) -> Self {
        Self { config, inner }
    }
}

#[async_trait::async_trait]
impl<T: IntoAcceptor> IntoAcceptor for TlsListener<T> {
    type Acceptor = TlsAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let cert = tokio_rustls::rustls::internal::pemfile::certs(&mut self.config.cert.as_slice())
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls certificates"))?;
        let key = {
            let mut pkcs8 = tokio_rustls::rustls::internal::pemfile::pkcs8_private_keys(
                &mut self.config.key.as_slice(),
            )
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls private keys"))?;
            if !pkcs8.is_empty() {
                pkcs8.remove(0)
            } else {
                let mut rsa = tokio_rustls::rustls::internal::pemfile::rsa_private_keys(
                    &mut self.config.key.as_slice(),
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

        let client_auth = match self.config.client_auth {
            TlsClientAuth::Off => NoClientAuth::new(),
            TlsClientAuth::Optional(trust_anchor) => {
                AllowAnyAnonymousOrAuthenticatedClient::new(read_trust_anchor(&trust_anchor)?)
            }
            TlsClientAuth::Required(trust_anchor) => {
                AllowAnyAuthenticatedClient::new(read_trust_anchor(&trust_anchor)?)
            }
        };

        let mut config = ServerConfig::new(client_auth);
        config
            .set_single_cert_with_ocsp_and_sct(cert, key, self.config.ocsp_resp, Vec::new())
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        config.set_protocols(&["h2".into(), "http/1.1".into()]);

        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        Ok(TlsAcceptor {
            acceptor,
            inner: self.inner.into_acceptor().await?,
        })
    }
}

/// A TLS or SSL protocol acceptor.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsAcceptor<T> {
    acceptor: tokio_rustls::TlsAcceptor,
    inner: T,
}

#[async_trait::async_trait]
impl<T: Acceptor> Acceptor for TlsAcceptor<T> {
    type Addr = T::Addr;
    type Io = TlsStream<T::Io>;

    fn local_addr(&self) -> IoResult<Vec<Self::Addr>> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, Self::Addr)> {
        let (stream, addr) = self.inner.accept().await?;
        let stream = self.acceptor.accept(stream).await?;
        Ok((stream, addr))
    }
}
