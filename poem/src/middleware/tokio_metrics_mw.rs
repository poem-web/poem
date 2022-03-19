use std::{collections::BTreeMap, sync::Arc, time::Duration};

use parking_lot::Mutex;
use serde::{ser::SerializeMap, Serialize, Serializer};
use tokio_metrics::{TaskMetrics, TaskMonitor};

use crate::{
    endpoint::make_sync, Endpoint, IntoResponse, Middleware, Request, Response, Result, RouteMethod,
};

#[derive(Clone, Default)]
struct Monitors(Arc<Mutex<BTreeMap<String, (TaskMonitor, Metrics)>>>);

impl Serialize for Monitors {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let monitors = self.0.lock();
        let mut s = serializer.serialize_map(Some(monitors.len()))?;
        for (path, (_, metrics)) in monitors.iter() {
            s.serialize_entry(path, metrics)?;
        }
        s.end()
    }
}

/// Middleware for metrics with [`tokio-metrics`](https://crates.io/crates/tokio-metrics) crate.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-metrics")))]
pub struct TokioMetrics {
    interval: Duration,
    monitors: Monitors,
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
            monitors: Default::default(),
        }
    }

    /// Window interval (defaults to 5 seconds)
    pub fn interval(self, interval: Duration) -> Self {
        Self { interval, ..self }
    }

    /// Create an endpoint for exporting metrics.
    pub fn exporter(&self) -> impl Endpoint {
        let monitors = self.monitors.clone();
        RouteMethod::new().get(make_sync(move |_| {
            serde_json::to_string(&monitors)
                .unwrap()
                .with_content_type("application/json")
        }))
    }
}

impl<E: Endpoint> Middleware<E> for TokioMetrics {
    type Output = TokioMetricsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        TokioMetricsEndpoint {
            inner: ep,
            interval: self.interval,
            monitors: self.monitors.clone(),
        }
    }
}

pub struct TokioMetricsEndpoint<E> {
    inner: E,
    interval: Duration,
    monitors: Monitors,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TokioMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let task_monitor = {
            let mut monitors = self.monitors.0.lock();
            let path = req.uri().path();

            match monitors.get(path) {
                Some((monitor, _)) => monitor.clone(),
                None => {
                    let task_monitor = TaskMonitor::new();
                    let weak_monitors = Arc::downgrade(&self.monitors.0);
                    let interval = self.interval;
                    let path = path.to_string();

                    monitors.insert(path.clone(), (task_monitor.clone(), Default::default()));
                    tokio::spawn({
                        let task_monitor = task_monitor.clone();
                        async move {
                            for current_metrics in task_monitor.intervals() {
                                match weak_monitors.upgrade() {
                                    Some(monitors) => {
                                        let mut monitors = monitors.lock();
                                        if let Some((_, metrics)) = monitors.get_mut(&path) {
                                            *metrics = current_metrics.into();
                                        } else {
                                            break;
                                        }
                                    }
                                    None => break,
                                }
                                tokio::time::sleep(interval).await;
                            }
                        }
                    });

                    task_monitor
                }
            }
        };

        Ok(task_monitor
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
