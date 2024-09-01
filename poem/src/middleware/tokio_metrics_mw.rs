use std::{sync::Arc, time::Duration};

use parking_lot::Mutex;
use serde::Serialize;
use tokio_metrics::{TaskMetrics, TaskMonitor};

use crate::{
    endpoint::make_sync, Endpoint, IntoResponse, Middleware, Request, Response, Result, RouteMethod,
};

/// Middleware for metrics with [`tokio-metrics`](https://crates.io/crates/tokio-metrics) crate.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-metrics")))]
pub struct TokioMetrics {
    interval: Duration,
    metrics: Arc<Mutex<Metrics>>,
}

impl Default for TokioMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioMetrics {
    /// Create a tokio metrics middleware.
    pub fn new() -> Self {
        Self {
            interval: Duration::from_secs(5),
            metrics: Default::default(),
        }
    }

    /// Window interval (defaults to 5 seconds)
    pub fn interval(self, interval: Duration) -> Self {
        Self { interval, ..self }
    }

    /// Create an endpoint for exporting metrics.
    pub fn exporter(&self) -> impl Endpoint {
        let metrics = self.metrics.clone();
        RouteMethod::new().get(make_sync(move |_| {
            serde_json::to_string(&*metrics.lock())
                .unwrap()
                .with_content_type("application/json")
        }))
    }
}

impl<E: Endpoint> Middleware<E> for TokioMetrics {
    type Output = TokioMetricsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        let monitor = TaskMonitor::new();
        let interval = self.interval;
        let metrics = self.metrics.clone();

        tokio::spawn({
            let monitor = monitor.clone();
            async move {
                let mut intervals = monitor.intervals();
                loop {
                    tokio::time::sleep(interval).await;
                    if let Some(m) = intervals.next() {
                        *metrics.lock() = m.into();
                    }
                }
            }
        });

        TokioMetricsEndpoint { inner: ep, monitor }
    }
}

/// Endpoint for the TokioMetrics middleware.
pub struct TokioMetricsEndpoint<E> {
    inner: E,
    monitor: TaskMonitor,
}

impl<E: Endpoint> Endpoint for TokioMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        Ok(self
            .monitor
            .instrument(self.inner.call(req))
            .await?
            .into_response())
    }
}

#[derive(Serialize, Default)]
struct Metrics {
    instrumented_count: u64,
    dropped_count: u64,
    first_poll_count: u64,
    total_first_poll_delay: Duration,
    total_idled_count: u64,
    total_idle_duration: Duration,
    total_scheduled_count: u64,
    total_scheduled_duration: Duration,
    total_poll_count: u64,
    total_poll_duration: Duration,
    total_fast_poll_count: u64,
    total_fast_poll_duration: Duration,
    total_slow_poll_count: u64,
    total_slow_poll_duration: Duration,
}

impl From<TaskMetrics> for Metrics {
    fn from(metrics: TaskMetrics) -> Self {
        Self {
            instrumented_count: metrics.instrumented_count,
            dropped_count: metrics.dropped_count,
            first_poll_count: metrics.first_poll_count,
            total_first_poll_delay: metrics.total_first_poll_delay,
            total_idled_count: metrics.total_idled_count,
            total_idle_duration: metrics.total_idle_duration,
            total_scheduled_count: metrics.total_scheduled_count,
            total_scheduled_duration: metrics.total_scheduled_duration,
            total_poll_count: metrics.total_poll_count,
            total_poll_duration: metrics.total_poll_duration,
            total_fast_poll_count: metrics.total_fast_poll_count,
            total_fast_poll_duration: metrics.total_fast_poll_duration,
            total_slow_poll_count: metrics.total_slow_poll_count,
            total_slow_poll_duration: metrics.total_slow_poll_duration,
        }
    }
}
