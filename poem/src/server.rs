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
    listener::{Acceptor, AcceptorExt, Listener},
    web::{LocalAddr, RemoteAddr},
    Endpoint, EndpointExt, IntoEndpoint, Response,
};

enum Either<L, A> {
    Listener(L),
    Acceptor(A),
}

/// An HTTP Server.
pub struct Server<L, A> {
    listener: Either<L, A>,
    name: Option<String>,
}

impl<L: Listener> Server<L, Infallible> {
    /// Use the specified listener to create an HTTP server.
    pub fn new(listener: L) -> Self {
        Self {
            listener: Either::Listener(listener),
            name: None,
        }
    }
}

impl<A: Acceptor> Server<Infallible, A> {
    /// Use the specified acceptor to create an HTTP server.
    pub fn new_with_acceptor(acceptor: A) -> Self {
        Self {
            listener: Either::Acceptor(acceptor),
            name: None,
        }
    }
}

impl<L, A> Server<L, A>
where
    L: Listener,
    L::Acceptor: 'static,
    A: Acceptor + 'static,
{
    /// Specify the name of the server, it is only used for logs.
    pub fn name(self, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
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
        let ep = Arc::new(ep.into_endpoint().map_to_response());
        let Server { listener, name } = self;
        let name = name.as_deref();
        let alive_connections = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let timeout_notify = Arc::new(Notify::new());

        let mut acceptor = match listener {
            Either::Listener(listener) => listener.into_acceptor().await?.boxed(),
            Either::Acceptor(acceptor) => acceptor.boxed(),
        };

        tokio::pin!(signal);

        for addr in acceptor.local_addr() {
            tracing::info!(name = name, addr = %addr, "listening");
        }
        tracing::info!(name = name, "server started");

        loop {
            tokio::select! {
                _ = &mut signal => {
                    if let Some(timeout) = timeout {
                        tracing::info!(
                            name = name,
                            timeout_in_seconds = timeout.as_secs_f32(),
                            "initiate graceful shutdown",
                        );

                        let timeout_notify = timeout_notify.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(timeout).await;
                            timeout_notify.notify_waiters();
                        });
                    } else {
                        tracing::info!(name = name, "initiate graceful shutdown");
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
            tracing::info!(name = name, "wait for all connections to close.");
            notify.notified().await;
        }

        tracing::info!(name = name, "server stopped");
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
                match ep.call((req, local_addr, remote_addr).into()).await {
                    Ok(resp) => Ok::<http::Response<_>, Infallible>(resp.into()),
                    Err(err) => Ok(err.as_response().into()),
                }
            }
        }
    });

    let conn = Http::new()
        .serve_connection(socket, service)
        .with_upgrades();
    let _ = conn.await;
}
