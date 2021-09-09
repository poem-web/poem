use std::{convert::Infallible, sync::Arc};

use hyper::server::conn::Http;
use tokio::io::{AsyncRead, AsyncWrite, Result as IoResult};

use crate::{
    listener::{Acceptor, IntoAcceptor},
    web::RemoteAddr,
    Endpoint, EndpointExt, IntoEndpoint, Request, Response,
};

/// An HTTP Server.
pub struct Server<T> {
    acceptor: T,
}

impl<T: Acceptor> Server<T> {
    /// Use the specified listener to create an HTTP server.
    pub async fn new<K: IntoAcceptor<Acceptor = T>>(acceptor: K) -> IoResult<Server<T>> {
        Ok(Self {
            acceptor: acceptor.into_acceptor().await?,
        })
    }

    /// Returns the local address that this server is bound to.
    pub fn local_addr(&self) -> IoResult<T::Addr> {
        self.acceptor.local_addr()
    }

    /// Run this server.
    pub async fn run(self, ep: impl IntoEndpoint) -> IoResult<()> {
        let ep = Arc::new(ep.into_endpoint().map_to_response());
        let mut acceptor = self.acceptor;

        loop {
            if let Ok((socket, remote_addr)) = acceptor.accept().await {
                tokio::spawn(serve_connection(
                    socket,
                    RemoteAddr::new(remote_addr),
                    ep.clone(),
                ));
            }
        }
    }
}

async fn serve_connection(
    socket: impl AsyncRead + AsyncWrite + Send + Unpin + 'static,
    remote_addr: RemoteAddr,
    ep: Arc<dyn Endpoint<Output = Response>>,
) {
    let service = hyper::service::service_fn({
        move |req: hyper::Request<hyper::Body>| {
            let ep = ep.clone();
            let remote_addr = remote_addr.clone();
            async move {
                let req = Request::from_hyper_request(req, remote_addr);
                let cookie_jar = req.cookie().clone();
                let mut resp = ep.call(req).await.into_hyper_response();
                // Appends cookies to response headers
                cookie_jar.append_delta_to_headers(resp.headers_mut());
                Ok::<_, Infallible>(resp)
            }
        }
    });
    let _ = Http::new()
        .serve_connection(socket, service)
        .with_upgrades()
        .await;
}
