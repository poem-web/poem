use std::time::Instant;

use libopentelemetry::{
    global,
    metrics::{Counter, Unit, ValueRecorder},
    Key,
};

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

const METHOD_KEY: Key = Key::from_static_str("request_method");
const PATH_KEY: Key = Key::from_static_str("request_path");
const STATUS_KEY: Key = Key::from_static_str("response_status_code");

/// Middleware for metrics with OpenTelemetry.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryMetrics {
    request_count: Counter<u64>,
    error_count: Counter<u64>,
    duration: ValueRecorder<f64>,
}

impl Default for OpenTelemetryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenTelemetryMetrics {
    /// Create `OpenTelemetryMetrics` middleware with `meter`.
    pub fn new() -> Self {
        let meter = global::meter("poem");
        Self {
            request_count: meter
                .u64_counter("poem_requests_count")
                .with_description("total request count (since start of service)")
                .init(),
            error_count: meter
                .u64_counter("poem_errors_count")
                .with_description("failed request count (since start of service)")
                .init(),
            duration: meter
                .f64_value_recorder("poem_request_duration_ms")
                .with_unit(Unit::new("milliseconds"))
                .with_description(
                    "request duration histogram (in milliseconds, since start of service)",
                )
                .init(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for OpenTelemetryMetrics {
    type Output = OpenTelemetryMetricsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        OpenTelemetryMetricsEndpoint {
            request_count: self.request_count.clone(),
            error_count: self.error_count.clone(),
            duration: self.duration.clone(),
            inner: ep,
        }
    }
}

/// Endpoint for OpenTelemetryMetrics middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryMetricsEndpoint<E> {
    request_count: Counter<u64>,
    error_count: Counter<u64>,
    duration: ValueRecorder<f64>,
    inner: E,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for OpenTelemetryMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut labels = Vec::with_capacity(3);
        labels.push(METHOD_KEY.string(req.method().to_string()));
        labels.push(PATH_KEY.string(req.uri().path().to_string()));

        let s = Instant::now();
        let resp = self.inner.call(req).await?.into_response();
        let elapsed = s.elapsed();

        labels.push(STATUS_KEY.i64(resp.status().as_u16() as i64));

        if resp.status().is_server_error() {
            self.error_count.add(1, &labels)
        }
        self.request_count.add(1, &labels);
        self.duration
            .record(elapsed.as_secs_f64() / 1000.0, &labels);

        Ok(resp)
    }
}
