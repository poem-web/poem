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
    time::Duration,
};

use crate::{
    listener::{Acceptor, Listener},
    web::{LocalAddr, RemoteAddr},
    Endpoint, EndpointExt, IntoEndpoint, Response,
};

/// An HTTP Server.
pub struct Server<T> {
    acceptor: T,
}

impl<T: Acceptor> Server<T> {
    /// Use the specified listener to create an HTTP server.
    pub async fn new<K: Listener<Acceptor = T>>(listener: K) -> IoResult<Server<T>> {
        Ok(Self {
            acceptor: listener.into_acceptor().await?,
        })
    }

    /// Use the specified acceptor to create an HTTP server.
    pub fn new_with_acceptor(acceptor: T) -> Self {
        Self { acceptor }
    }

    /// Returns the local address that this server is bound to.
    pub fn local_addr(&self) -> Vec<LocalAddr> {
        self.acceptor.local_addr()
    }

    /// Run this server.
    pub async fn run<E>(self, ep: E) -> IoResult<()>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.run_with_graceful_shutdown(ep, futures_util::future::pending(), None)
            .await
    }

    /// Run this server and a signal to initiate graceful shutdown.
    pub async fn run_with_graceful_shutdown<E>(
        self,
        ep: E,
        signal: impl Future<Output = ()>,
        timeout: Option<Duration>,
    ) -> IoResult<()>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        let ep = ep.into_endpoint();
        let ep = Arc::new(ep.map_to_response());
        let Server { mut acceptor } = self;
        let alive_connections = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let timeout_notify = Arc::new(Notify::new());

        tokio::pin!(signal);

        for addr in acceptor.local_addr() {
            tracing::info!(addr = %addr, "listening");
        }
        tracing::info!("server started");

        loop {
            tokio::select! {
                _ = &mut signal => {
                    if let Some(timeout) = timeout {
                        tracing::info!(
                            timeout_in_seconds = timeout.as_secs_f32(),
                            "initiate graceful shutdown",
                        );

                        let timeout_notify = timeout_notify.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(timeout).await;
                            timeout_notify.notify_waiters();
                        });
                    } else {
                        tracing::info!("initiate graceful shutdown");
                    }
                    break;
                },
                res = acceptor.accept() => {
                    if let Ok((socket, local_addr, remote_addr)) = res {
                        let ep = ep.clone();
                        let alive_connections = alive_connections.clone();
                        let notify = notify.clone();
                        let timeout_notify = timeout_notify.clone();

                        tokio::spawn(async move {
                            alive_connections.fetch_add(1, Ordering::SeqCst);

                            if timeout.is_some() {
                                tokio::select! {
                                    _ = serve_connection(socket, local_addr, remote_addr, ep) => {}
                                    _ = timeout_notify.notified() => {}
                                }
                            } else {
                                serve_connection(socket, local_addr, remote_addr, ep).await;
                            }

                            if alive_connections.fetch_sub(1, Ordering::SeqCst) == 1 {
                                notify.notify_one();
                            }
                        });
                    }
                }
            }
        }

        drop(acceptor);
        if alive_connections.load(Ordering::SeqCst) > 0 {
            tracing::info!("wait for all connections to close.");
            notify.notified().await;
        }

        tracing::info!("server stopped");
        Ok(())
    }
}

async fn serve_connection(
    socket: impl AsyncRead + AsyncWrite + Send + Unpin + 'static,
    local_addr: LocalAddr,
    remote_addr: RemoteAddr,
    ep: Arc<dyn Endpoint<Output = Response>>,
) {
    let service = hyper::service::service_fn({
        move |req: hyper::Request<hyper::Body>| {
            let ep = ep.clone();
            let local_addr = local_addr.clone();
            let remote_addr = remote_addr.clone();
            async move {
                let resp = ep.call((req, local_addr, remote_addr).into()).await.into();
                Ok::<_, Infallible>(resp)
            }
        }
    });

    let conn = Http::new()
        .serve_connection(socket, service)
        .with_upgrades();
    let _ = conn.await;
}
