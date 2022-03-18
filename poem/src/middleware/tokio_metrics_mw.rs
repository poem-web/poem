use http::Method;
use tokio_metrics::TaskMonitor;

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Middleware for metrics with [`tokio-metrics`](https://crates.io/crates/tokio-metrics) crate.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-metrics")))]
pub struct TokioMetrics {
    task_monitor: TaskMonitor,
    exporter_path: String,
}

impl TokioMetrics {
    /// Create a tokio metrics middleware.
    pub fn new(exporter_path: impl Into<String>) -> Self {
        Self {
            task_monitor: TaskMonitor::new(),
            exporter_path: exporter_path.into(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for TokioMetrics {
    type Output = TokioMetricsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        TokioMetricsEndpoint {
            inner: ep,
            task_monitor: self.task_monitor.clone(),
            exporter_path: self.exporter_path.clone(),
        }
    }
}

pub struct TokioMetricsEndpoint<E> {
    inner: E,
    task_monitor: TaskMonitor,
    exporter_path: String,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TokioMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() == Method::GET && req.uri().path() == self.exporter_path {
            return Ok(format!("{:?}", self.task_monitor.cumulative()).into_response());
        }
        Ok(self
            .task_monitor
            .instrument(self.inner.call(req))
            .await?
            .into_response())
    }
}
