use std::time::Instant;

use tracing::{Instrument, Level};

use crate::{
    route::PathPattern, web::RealIp, Endpoint, FromRequest, IntoResponse, Middleware, Request,
    Response, Result,
};

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
        let remote_addr = RealIp::from_request_without_body(&req)
            .await
            .ok()
            .and_then(|real_ip| real_ip.0)
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| req.remote_addr().to_string());

        let span = tracing::span!(
            target: module_path!(),
            Level::INFO,
            "request",
            remote_addr = %remote_addr,
            version = ?req.version(),
            method = %req.method(),
            uri = %req.original_uri(),
        );

        if let Some(path_pattern) = req.data::<PathPattern>() {
            span.record("path_pattern", path_pattern.0.as_ref());
        }

        async move {
            let now = Instant::now();
            let res = self.inner.call(req).await;
            let duration = now.elapsed();

            match res {
                Ok(resp) => {
                    let resp = resp.into_response();
                    tracing::info!(
                        status = %resp.status(),
                        duration = ?duration,
                        "response"
                    );
                    Ok(resp)
                }
                Err(err) => {
                    tracing::info!(
                        status = %err.status(),
                        error = %err,
                        duration = ?duration,
                        "error"
                    );
                    Err(err)
                }
            }
        }
        .instrument(span)
        .await
    }
}
