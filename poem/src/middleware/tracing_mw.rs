use std::time::SystemTime;

use tracing::{Instrument, Level};

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Middleware for [`tracing`](https://crates.io/crates/tracing).
#[derive(Default)]
pub struct Tracing;

impl<E: Endpoint> Middleware<E> for Tracing {
    type Output = TracingEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        TracingEndpoint { inner: ep }
    }
}

/// Endpoint for `Tracing` middleware.
pub struct TracingEndpoint<E> {
    inner: E,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TracingEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let span = tracing::span!(
            target: module_path!(),
            Level::INFO,
            "request",
            remote_addr = %req.remote_addr(),
            version = ?req.version(),
            method = %req.method(),
            path = %req.uri(),
        );

        async move {
            let now = SystemTime::now();
            let resp = self.inner.call(req).await?.into_response();
            match now.elapsed() {
                Ok(duration) => tracing::info!(
                    status = %resp.status(),
                    duration = ?duration,
                    "response"
                ),
                Err(_) => tracing::info!(
                    status = %resp.status(),
                    "response"
                ),
            }
            Ok(resp)
        }
        .instrument(span)
        .await
    }
}
