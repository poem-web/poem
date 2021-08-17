use tracing::{Instrument, Level};

use super::Middleware;
use crate::{Endpoint, Request, Response, Result};

/// A middleware for tracing requests and responses
pub struct Tracing;

impl<E: Endpoint> Middleware<E> for Tracing {
    type Output = TracingImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        TracingImpl { inner: ep }
    }
}

#[doc(hidden)]
pub struct TracingImpl<E> {
    inner: E,
}

#[async_trait::async_trait]
impl<E> Endpoint for TracingImpl<E>
where
    E: Endpoint,
{
    async fn call(&self, req: Request) -> Result<Response> {
        let span = tracing::span!(
            Level::INFO,
            "handle request",
            method = %req.method(),
            path = %req.uri(),
        );

        let fut = self.inner.call(req);
        async move {
            let resp = fut.await;

            match &resp {
                Ok(resp) => ::tracing::info!(status = %resp.status(), "send response"),
                Err(err) => ::tracing::error!(error = %err, "an error occurred"),
            }

            resp
        }
        .instrument(span)
        .await
    }
}
