use std::{convert::Infallible, sync::Arc};

use hyper::server::conn::Http;
use tokio::{
    io::{AsyncRead, AsyncWrite, Result as IoResult},
    net::{TcpListener, ToSocketAddrs},
};
#[cfg(feature = "tls")]
use tokio_rustls::{
    rustls::{
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth,
        RootCertStore, ServerConfig,
    },
    TlsAcceptor,
};

use crate::{Endpoint, Request};

/// An HTTP Server.
pub struct Server {
    ep: Arc<dyn Endpoint>,
}

impl Server {
    /// Run this server.
    pub async fn run(self, addr: impl ToSocketAddrs) -> IoResult<()> {
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;
            tokio::spawn(serve_connection(socket, self.ep.clone()));
        }
    }

    /// Configure a server to use TLS.
    #[cfg(feature = "tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
    pub fn tls(self) -> TlsServer {
        TlsServer {
            ep: self.ep,
            cert: Vec::new(),
            key: Vec::new(),
            client_auth: TlsClientAuth::Off,
            ocsp_resp: Vec::new(),
        }
    }
}

/// Create a Server with the endpoint.
pub fn serve(ep: impl Endpoint) -> Server {
    Server { ep: Arc::new(ep) }
}

#[cfg(feature = "tls")]
enum TlsClientAuth {
    Off,
    Optional(Vec<u8>),
    Required(Vec<u8>),
}

/// An HTTP Server over TLS.
#[cfg(feature = "tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsServer {
    ep: Arc<dyn Endpoint>,
    cert: Vec<u8>,
    key: Vec<u8>,
    client_auth: TlsClientAuth,
    ocsp_resp: Vec<u8>,
}

#[cfg(feature = "tls")]
impl TlsServer {
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

    /// Run this server.
    pub async fn run(self, addr: impl ToSocketAddrs) -> IoResult<()> {
        use std::io::{Error as IoError, ErrorKind};

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

        let client_auth = match self.client_auth {
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
            .set_single_cert_with_ocsp_and_sct(cert, key, self.ocsp_resp, Vec::new())
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        config.set_protocols(&["h2".into(), "http/1.1".into()]);

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let ep = self.ep.clone();
            tokio::spawn(async move {
                if let Ok(tls_socket) = acceptor.accept(socket).await {
                    serve_connection(tls_socket, ep).await;
                }
            });
        }
    }
}

async fn serve_connection(
    socket: impl AsyncRead + AsyncWrite + Unpin + 'static,
    ep: Arc<dyn Endpoint>,
) {
    let service = hyper::service::service_fn({
        move |req: hyper::Request<hyper::Body>| {
            let ep = ep.clone();
            async move {
                let req = match Request::from_http_request(req) {
                    Ok(req) => req,
                    Err(err) => return Ok(err.as_response().into_http_response()),
                };

                let resp = match ep.call(req).await {
                    Ok(resp) => resp.into_http_response(),
                    Err(err) => err.as_response().into_http_response(),
                };
                Ok::<_, Infallible>(resp)
            }
        }
    });
    let _ = Http::new().serve_connection(socket, service).await;
}
