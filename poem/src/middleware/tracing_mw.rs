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
            let res = self.inner.call(req).await;

            match (res, now.elapsed()) {
                (Ok(resp), Ok(duration)) => {
                    let resp = resp.into_response();
                    tracing::info!(
                        status = %resp.status(),
                        duration = ?duration,
                        "response"
                    );
                    Ok(resp)
                }
                (Ok(resp), Err(_)) => {
                    let resp = resp.into_response();
                    tracing::info!(
                        status = %resp.status(),
                        "response"
                    );
                    Ok(resp)
                }
                (Err(err), _) => {
                    tracing::info!(error = %err, "error");
                    Err(err)
                }
            }
        }
        .instrument(span)
        .await
    }
}
