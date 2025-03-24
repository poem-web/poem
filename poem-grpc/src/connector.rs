use std::{
    io::{Error as IoError, Result as IoResult},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::{FutureExt, future::BoxFuture};
use http::{Uri, uri::Scheme};
use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::{
    client::legacy::connect::{Connected, Connection},
    rt::TokioIo,
};
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};
use tower_service::Service;

pub(crate) enum MaybeHttpsStream {
    TcpStream(TokioIo<TcpStream>),
    TlsStream {
        stream: TokioIo<TlsStream<TcpStream>>,
        is_http2: bool,
    },
}

impl Read for MaybeHttpsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<IoResult<()>> {
        match self.get_mut() {
            MaybeHttpsStream::TcpStream(stream) => Pin::new(stream).poll_read(cx, buf),
            MaybeHttpsStream::TlsStream { stream, .. } => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl Write for MaybeHttpsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        match self.get_mut() {
            MaybeHttpsStream::TcpStream(stream) => Pin::new(stream).poll_write(cx, buf),
            MaybeHttpsStream::TlsStream { stream, .. } => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        match self.get_mut() {
            MaybeHttpsStream::TcpStream(stream) => Pin::new(stream).poll_flush(cx),
            MaybeHttpsStream::TlsStream { stream, .. } => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        match self.get_mut() {
            MaybeHttpsStream::TcpStream(stream) => Pin::new(stream).poll_shutdown(cx),
            MaybeHttpsStream::TlsStream { stream, .. } => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

impl Connection for MaybeHttpsStream {
    fn connected(&self) -> Connected {
        match self {
            MaybeHttpsStream::TcpStream(_) => Connected::new(),
            MaybeHttpsStream::TlsStream { is_http2, .. } => {
                let mut connected = Connected::new();
                if *is_http2 {
                    connected = connected.negotiated_h2();
                }
                connected
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpsConnector {
    tls_config: Option<ClientConfig>,
}

impl HttpsConnector {
    #[inline]
    pub(crate) fn new(tls_config: Option<ClientConfig>) -> Self {
        HttpsConnector { tls_config }
    }
}

impl Service<Uri> for HttpsConnector {
    type Response = MaybeHttpsStream;
    type Error = IoError;
    type Future = BoxFuture<'static, Result<MaybeHttpsStream, IoError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        do_connect(uri, self.tls_config.clone()).boxed()
    }
}

fn default_tls_config() -> ClientConfig {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth()
}

async fn do_connect(
    uri: Uri,
    tls_config: Option<ClientConfig>,
) -> Result<MaybeHttpsStream, IoError> {
    let scheme = uri
        .scheme()
        .ok_or_else(|| IoError::other("missing scheme"))?
        .clone();
    let host = uri
        .host()
        .ok_or_else(|| IoError::other("missing host"))?
        .to_string();
    let port = uri
        .port_u16()
        .unwrap_or_else(|| if scheme == Scheme::HTTPS { 443 } else { 80 });

    if scheme == Scheme::HTTP {
        let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        Ok(MaybeHttpsStream::TcpStream(TokioIo::new(stream)))
    } else if scheme == Scheme::HTTPS {
        let mut tls_config = tls_config.unwrap_or_else(default_tls_config);
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let connector = TlsConnector::from(Arc::new(tls_config));
        let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let domain = host.try_into().map_err(IoError::other)?;
        let mut is_http2 = false;
        let stream = connector
            .connect_with(domain, stream, |conn| {
                is_http2 = conn.alpn_protocol() == Some(b"h2");
            })
            .await?;
        Ok(MaybeHttpsStream::TlsStream {
            stream: TokioIo::new(stream),
            is_http2,
        })
    } else {
        Err(IoError::other("invalid scheme"))
    }
}
