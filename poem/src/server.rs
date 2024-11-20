use std::{
    convert::Infallible,
    future::Future,
    io,
    io::IoSlice,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures_util::FutureExt;
use http::uri::Scheme;
use hyper::body::Incoming;
use hyper_util::server::conn::auto;
use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf, Result as IoResult},
    sync::{oneshot, Notify},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

use crate::{
    endpoint::{DynEndpoint, ToDynEndpoint},
    listener::{Acceptor, AcceptorExt, Listener},
    web::{LocalAddr, RemoteAddr},
    Endpoint, EndpointExt, IntoEndpoint, Response,
};

enum Either<L, A> {
    Listener(L),
    Acceptor(A),
}

/// An HTTP Server.
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub struct Server<L, A> {
    listener: Either<L, A>,
    name: Option<String>,
    idle_timeout: Option<Duration>,
    http2_max_concurrent_streams: Option<u32>,
    http2_max_pending_accept_reset_streams: Option<u32>,
    http2_max_header_list_size: u32,
}

impl<L: Listener> Server<L, Infallible> {
    /// Use the specified listener to create an HTTP server.
    pub fn new(listener: L) -> Self {
        Self {
            listener: Either::Listener(listener),
            name: None,
            idle_timeout: None,
            http2_max_concurrent_streams: None,
            http2_max_pending_accept_reset_streams: Some(20),
            http2_max_header_list_size: 16384,
        }
    }
}

impl<A: Acceptor> Server<Infallible, A> {
    /// Use the specified acceptor to create an HTTP server.
    pub fn new_with_acceptor(acceptor: A) -> Self {
        Self {
            listener: Either::Acceptor(acceptor),
            name: None,
            idle_timeout: None,
            http2_max_concurrent_streams: None,
            http2_max_pending_accept_reset_streams: Some(20),
            http2_max_header_list_size: 16384,
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
    #[must_use]
    pub fn name(self, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    /// Specify connection idle timeout. Connections will be terminated if there
    /// was no activity within this period of time
    #[must_use]
    pub fn idle_timeout(self, timeout: Duration) -> Self {
        Self {
            idle_timeout: Some(timeout),
            ..self
        }
    }

    /// Sets the [`SETTINGS_MAX_CONCURRENT_STREAMS`][spec] option for HTTP2
    /// connections.
    ///
    /// Default is 200. Passing `None` will remove any limit.
    ///
    /// [spec]: https://http2.github.io/http2-spec/#SETTINGS_MAX_CONCURRENT_STREAMS
    pub fn http2_max_concurrent_streams(self, max: impl Into<Option<u32>>) -> Self {
        Self {
            http2_max_concurrent_streams: max.into(),
            ..self
        }
    }

    /// Sets the max size of received header frames.
    ///
    /// Default is `16384` bytes.
    pub fn http2_max_header_list_size(self, max: u32) -> Self {
        Self {
            http2_max_header_list_size: max,
            ..self
        }
    }

    /// Configures the maximum number of pending reset streams allowed before a
    /// GOAWAY will be sent.
    ///
    /// This will default to the default value set by the [`h2` crate](https://crates.io/crates/h2).
    /// As of v0.4.0, it is 20.
    pub fn http2_max_pending_accept_reset_streams(self, max: impl Into<Option<u32>>) -> Self {
        Self {
            http2_max_pending_accept_reset_streams: max.into(),
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
        let ep = Arc::new(ToDynEndpoint(ep.into_endpoint().map_to_response()));
        let Server {
            listener,
            name,
            idle_timeout,
            http2_max_concurrent_streams,
            http2_max_pending_accept_reset_streams,
            http2_max_header_list_size,
        } = self;
        let name = name.as_deref();
        let alive_connections = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let timeout_token = CancellationToken::new();
        let server_graceful_shutdown_token = CancellationToken::new();

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
                    server_graceful_shutdown_token.cancel();
                    if let Some(timeout) = timeout {
                        tracing::info!(
                            name = name,
                            timeout_in_seconds = timeout.as_secs_f32(),
                            "initiate graceful shutdown",
                        );

                        let timeout_token = timeout_token.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(timeout).await;
                            timeout_token.cancel();
                        });
                    } else {
                        tracing::info!(name = name, "initiate graceful shutdown");
                    }
                    break;
                },
                res = acceptor.accept() => {
                    if let Ok((socket, local_addr, remote_addr, scheme)) = res {
                        alive_connections.fetch_add(1, Ordering::Release);

                        let ep = ep.clone();
                        let alive_connections = alive_connections.clone();
                        let notify = notify.clone();
                        let timeout_token = timeout_token.clone();
                        let server_graceful_shutdown_token = server_graceful_shutdown_token.clone();
                        let server_graceful_shutdown_token_clone = server_graceful_shutdown_token.clone();

                        let spawn_fut = AssertUnwindSafe(async move {
                            let serve_connection = serve_connection(ConnectionOptions{
                                socket,
                                local_addr,
                                remote_addr,
                                scheme,
                                ep,
                                server_graceful_shutdown_token: server_graceful_shutdown_token.clone(),
                                idle_connection_close_timeout: idle_timeout,
                                http2_max_concurrent_streams,
                                http2_max_pending_accept_reset_streams,
                                http2_max_header_list_size,
                            });

                            if timeout.is_some() {
                                tokio::select! {
                                    _ = serve_connection => {}
                                    _ = timeout_token.cancelled() => {}
                                }
                            } else {
                               serve_connection.await;
                            }
                        });

                        tokio::spawn(async move {
                            let result = spawn_fut.catch_unwind().await;

                            if alive_connections.fetch_sub(1, Ordering::Acquire) == 1 {
                                // notify only if shutdown is initiated, to prevent notification when server is active.
                                // It's a valid state to have 0 alive connections when server is not shutting down.
                                if server_graceful_shutdown_token_clone.is_cancelled() {
                                    notify.notify_one();
                                }
                            }

                            if let Err(err) = result {
                                std::panic::resume_unwind(err);
                            }
                        });
                    }
                }
            }
        }

        drop(acceptor);
        if alive_connections.load(Ordering::Acquire) > 0 {
            tracing::info!(name = name, "wait for all connections to close.");
            notify.notified().await;
        }

        tracing::info!(name = name, "server stopped");
        Ok(())
    }
}

pin_project! {
    struct ClosingInactiveConnection<T> {
        #[pin]
        inner: T,
        #[pin]
        alive: Arc<Notify>,
        timeout: Duration,
        stop_tx: oneshot::Sender<()>,
    }
}

impl<T> AsyncRead for ClosingInactiveConnection<T>
where
    T: AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();

        match this.inner.poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                this.alive.notify_waiters();
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> AsyncWrite for ClosingInactiveConnection<T>
where
    T: AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        this.alive.notify_waiters();
        this.inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.alive.notify_waiters();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        this.alive.notify_waiters();
        this.inner.poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        this.alive.notify_waiters();
        this.inner.poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}

impl<T> ClosingInactiveConnection<T> {
    fn new<F, Fut>(inner: T, timeout: Duration, mut f: F) -> ClosingInactiveConnection<T>
    where
        F: Send + FnMut() -> Fut + 'static,
        Fut: Future + Send + 'static,
    {
        let alive = Arc::new(Notify::new());
        let (stop_tx, stop_rx) = oneshot::channel();
        tokio::spawn({
            let alive = alive.clone();

            async move {
                let check_timeout = async {
                    loop {
                        match tokio::time::timeout(timeout, alive.notified()).await {
                            Ok(()) => {}
                            Err(_) => {
                                f().await;
                            }
                        }
                    }
                };
                tokio::select! {
                    _ = stop_rx => {},
                    _ = check_timeout => {}
                }
            }
        });
        Self {
            inner,
            alive,
            timeout,
            stop_tx,
        }
    }
}

struct ConnectionOptions<Io> {
    socket: Io,
    local_addr: LocalAddr,
    remote_addr: RemoteAddr,
    scheme: Scheme,
    ep: Arc<dyn DynEndpoint<Output = Response>>,
    server_graceful_shutdown_token: CancellationToken,
    idle_connection_close_timeout: Option<Duration>,
    http2_max_concurrent_streams: Option<u32>,
    http2_max_pending_accept_reset_streams: Option<u32>,
    http2_max_header_list_size: u32,
}

async fn serve_connection<Io>(opts: ConnectionOptions<Io>)
where
    Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let ConnectionOptions {
        socket,
        local_addr,
        remote_addr,
        scheme,
        ep,
        server_graceful_shutdown_token,
        idle_connection_close_timeout,
        http2_max_concurrent_streams,
        http2_max_pending_accept_reset_streams,
        http2_max_header_list_size,
    } = opts;

    let connection_shutdown_token = CancellationToken::new();

    let service = hyper::service::service_fn({
        let remote_addr = remote_addr.clone();

        move |req: http::Request<Incoming>| {
            let ep = ep.clone();
            let local_addr = local_addr.clone();
            let remote_addr = remote_addr.clone();
            let scheme = scheme.clone();
            async move {
                Ok::<http::Response<_>, Infallible>(
                    ep.get_response((req, local_addr, remote_addr, scheme).into())
                        .await
                        .into(),
                )
            }
        }
    });

    let socket = match idle_connection_close_timeout {
        Some(timeout) => {
            tokio_util::either::Either::Left(ClosingInactiveConnection::new(socket, timeout, {
                let connection_shutdown_token = connection_shutdown_token.clone();

                move || {
                    let connection_shutdown_token = connection_shutdown_token.clone();
                    async move {
                        connection_shutdown_token.cancel();
                    }
                }
            }))
        }
        None => tokio_util::either::Either::Right(socket),
    };

    let mut builder = auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    let mut builder = builder.http2();
    let builder = builder
        .max_concurrent_streams(http2_max_concurrent_streams)
        .max_pending_accept_reset_streams(
            http2_max_pending_accept_reset_streams.map(|x| x as usize),
        )
        .max_header_list_size(http2_max_header_list_size);

    let conn =
        builder.serve_connection_with_upgrades(hyper_util::rt::TokioIo::new(socket), service);
    futures_util::pin_mut!(conn);

    tokio::select! {
        _ = &mut conn => {
            // Connection completed successfully.
        },
        _ = connection_shutdown_token.cancelled() => {
            tracing::info!(remote_addr=%remote_addr, "closing connection due to inactivity");
        }
        _ = server_graceful_shutdown_token.cancelled() => {}
    }

    // Init graceful shutdown for connection
    conn.as_mut().graceful_shutdown();
    // Continue awaiting after graceful-shutdown is initiated to handle existed
    // requests.
    let _ = conn.await;
}
