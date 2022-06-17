use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures_util::{
    stream::{BoxStream, Chain, Pending},
    Stream, StreamExt,
};
use http::uri::Scheme;
use openssl::{
    pkey::PKey,
    ssl::{Ssl, SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod, SslRef},
    x509::X509,
};
use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_openssl::SslStream;
use tokio_util::either::Either;

use crate::{
    listener::{Acceptor, HandshakeStream, IntoTlsConfigStream, Listener},
    web::{LocalAddr, RemoteAddr},
};

/// Openssl configuration contains certificate's chain and private key.
pub struct OpensslTlsConfig {
    cert: Either<Vec<u8>, PathBuf>,
    key: Either<Vec<u8>, PathBuf>,
}

impl Default for OpensslTlsConfig {
    fn default() -> Self {
        Self {
            cert: Either::Left(vec![]),
            key: Either::Left(vec![]),
        }
    }
}

impl OpensslTlsConfig {
    /// Creates new Openssl TLS configuration.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets certificate's chain from PEM data.
    pub fn cert_from_data(mut self, cert_data: impl Into<Vec<u8>>) -> Self {
        self.cert = Either::Left(cert_data.into());
        self
    }

    /// Sets file path to certificate's chain in PEM format.
    pub fn cert_from_file(mut self, cert_file: impl AsRef<Path>) -> Self {
        self.cert = Either::Right(cert_file.as_ref().to_owned());
        self
    }

    /// Sets private key from PEM data.
    pub fn key_from_data(mut self, key_data: impl Into<Vec<u8>>) -> Self {
        self.key = Either::Left(key_data.into());
        self
    }

    /// Sets file path to private key in PEM format.
    pub fn key_from_file(mut self, key_file: impl AsRef<Path>) -> Self {
        self.key = Either::Right(key_file.as_ref().to_owned());
        self
    }

    fn create_acceptor_builder(&self) -> IoResult<SslAcceptorBuilder> {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        match &self.cert {
            Either::Left(data) => {
                let mut certs = X509::stack_from_pem(data)?;
                let mut certs = certs.drain(..);
                builder.set_certificate(
                    certs
                        .next()
                        .ok_or_else(|| IoError::new(ErrorKind::Other, "no leaf certificate"))?
                        .as_ref(),
                )?;
                certs.try_for_each(|cert| builder.add_extra_chain_cert(cert))?;
            }
            Either::Right(path) => builder.set_certificate_chain_file(path)?,
        }
        match &self.key {
            Either::Left(data) => {
                builder.set_private_key(PKey::private_key_from_pem(data)?.as_ref())?
            }
            Either::Right(path) => builder.set_private_key_file(path, SslFiletype::PEM)?,
        }

        // set ALPN protocols
        static PROTOS: &[u8] = b"\x02h2\x08http/1.1";
        builder.set_alpn_protos(PROTOS)?;
        // set uo ALPN selection routine - as select_next_proto
        builder.set_alpn_select_callback(move |_: &mut SslRef, list: &[u8]| {
            openssl::ssl::select_next_proto(PROTOS, list).ok_or(openssl::ssl::AlpnError::NOACK)
        });
        Ok(builder)
    }
}

impl<T> IntoTlsConfigStream<OpensslTlsConfig> for T
where
    T: Stream<Item = OpensslTlsConfig> + Send + 'static,
{
    type Stream = Self;

    fn into_stream(self) -> IoResult<Self::Stream> {
        Ok(self)
    }
}

impl IntoTlsConfigStream<OpensslTlsConfig> for OpensslTlsConfig {
    type Stream = futures_util::stream::Once<futures_util::future::Ready<OpensslTlsConfig>>;

    fn into_stream(self) -> IoResult<Self::Stream> {
        let _ = self.create_acceptor_builder()?;
        Ok(futures_util::stream::once(futures_util::future::ready(
            self,
        )))
    }
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol with [`openssl-tls`](https://crates.io/crates/openssl).
///
/// NOTE: You cannot create it directly and should use the
/// [`native_tls`](crate::listener::Listener::openssl_tls) method to create it,
/// because it needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
pub struct OpensslTlsListener<T, S> {
    inner: T,
    config_stream: S,
}

impl<T, S> OpensslTlsListener<T, S>
where
    T: Listener,
    S: IntoTlsConfigStream<OpensslTlsConfig>,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        Self {
            inner,
            config_stream,
        }
    }
}

#[async_trait::async_trait]
impl<T: Listener, S: IntoTlsConfigStream<OpensslTlsConfig>> Listener for OpensslTlsListener<T, S> {
    type Acceptor = OpensslTlsAcceptor<T::Acceptor, BoxStream<'static, OpensslTlsConfig>>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        Ok(OpensslTlsAcceptor::new(
            self.inner.into_acceptor().await?,
            self.config_stream.into_stream()?.boxed(),
        ))
    }
}

/// A TLS or SSL protocol acceptor with [`native_tls`](https://crates.io/crates/openssl).
#[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
pub struct OpensslTlsAcceptor<T, S> {
    inner: T,
    config_stream: Chain<S, Pending<OpensslTlsConfig>>,
    current_tls_acceptor: Option<Arc<SslAcceptor>>,
}

impl<T, S> OpensslTlsAcceptor<T, S>
where
    S: Stream<Item = OpensslTlsConfig> + Send + Unpin + 'static,
{
    pub(crate) fn new(inner: T, config_stream: S) -> Self {
        OpensslTlsAcceptor {
            inner,
            config_stream: config_stream.chain(futures_util::stream::pending()),
            current_tls_acceptor: None,
        }
    }
}

#[async_trait::async_trait]
impl<T, S> Acceptor for OpensslTlsAcceptor<T, S>
where
    S: Stream<Item = OpensslTlsConfig> + Send + Unpin + 'static,
    T: Acceptor,
{
    type Io = HandshakeStream<SslStream<T::Io>>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        loop {
            tokio::select! {
                res = self.config_stream.next() => {
                    if let Some(tls_config) = res {
                        match tls_config.create_acceptor_builder() {
                            Ok(builder) => {
                                if self.current_tls_acceptor.is_some() {
                                    tracing::info!("tls config changed.");
                                } else {
                                    tracing::info!("tls config loaded.");
                                }
                                self.current_tls_acceptor = Some(Arc::new(builder.build()));
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
                        Some(tls_acceptor) => tls_acceptor.clone(),
                        None => return Err(IoError::new(ErrorKind::Other, "no valid tls config.")),
                    };
                    let fut = async move {
                        let ssl = Ssl::new(tls_acceptor.context()).map_err(|err|
                            IoError::new(ErrorKind::Other, err.to_string()))?;
                        let mut tls_stream = SslStream::new(ssl, stream).map_err(|err|
                            IoError::new(ErrorKind::Other, err.to_string()))?;
                        use std::pin::Pin;
                        Pin::new(&mut tls_stream).accept().await.map_err(|err|
                            IoError::new(ErrorKind::Other, err.to_string()))?;
                        Ok(tls_stream) };
                    let stream = HandshakeStream::new(fut);
                    return Ok((stream, local_addr, remote_addr, Scheme::HTTPS));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use openssl::ssl::SslConnector;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use super::*;
    use crate::listener::TcpListener;

    #[tokio::test]
    async fn tls_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").openssl_tls(
            OpensslTlsConfig::new()
                .cert_from_file("src/listener/certs/cert1.pem")
                .key_from_file("src/listener/certs/key1.pem"),
        );
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let local_addr = acceptor.local_addr().pop().unwrap();

        tokio::spawn(async move {
            let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();
            connector
                .set_ca_file("src/listener/certs/chain1.pem")
                .unwrap();

            let ssl = connector
                .build()
                .configure()
                .unwrap()
                .into_ssl("testserver.com")
                .unwrap();

            let stream = TcpStream::connect(local_addr.as_socket_addr().unwrap())
                .await
                .unwrap();
            let mut tls_stream = SslStream::new(ssl, stream).unwrap();
            use std::pin::Pin;
            Pin::new(&mut tls_stream).connect().await.unwrap();

            tls_stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _, _, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
