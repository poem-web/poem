use std::{
    convert::Infallible,
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use hyper::server::conn::Http;
use tokio::{
    io::{AsyncRead, AsyncWrite, Result as IoResult},
    sync::Notify,
};

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
    pub fn local_addr(&self) -> IoResult<Vec<T::Addr>> {
        self.acceptor.local_addr()
    }

    /// Run this server.
    pub async fn run(self, ep: impl IntoEndpoint) -> IoResult<()> {
        self.run_with_graceful_shutdown(ep, futures_util::future::pending())
            .await
    }

    /// Run this server and a signal to initiate graceful shutdown.
    pub async fn run_with_graceful_shutdown(
        self,
        ep: impl IntoEndpoint,
        signal: impl Future<Output = ()>,
    ) -> IoResult<()> {
        let ep = Arc::new(ep.into_endpoint().map_to_response());
        let mut acceptor = self.acceptor;
        let alive_connections = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());

        tokio::pin!(signal);

        loop {
            tokio::select! {
                _ = &mut signal => break,
                res = acceptor.accept() => {
                    if let Ok((socket, remote_addr)) = res {
                        let ep = ep.clone();
                        let alive_connections = alive_connections.clone();
                        let notify = notify.clone();
                        tokio::spawn(async move {
                            alive_connections.fetch_add(1, Ordering::SeqCst);
                            serve_connection(socket, RemoteAddr::new(remote_addr), ep).await;
                            if alive_connections.fetch_sub(1, Ordering::SeqCst) == 1 {
                                notify.notify_one();
                            }
                        });
                    }
                }
            }
        }

        if alive_connections.load(Ordering::SeqCst) > 0 {
            notify.notified().await;
        }
        Ok(())
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
                let req: Request = (req, remote_addr).into();
                let cookie_jar = req.cookie().clone();
                let mut resp: http::Response<hyper::Body> = ep.call(req).await.into();
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
