use std::time::Instant;

use libopentelemetry::{
    global,
    metrics::{Counter, Histogram, Unit},
    Context, Key,
};
use opentelemetry_semantic_conventions::trace;

use crate::{route::PathPattern, Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Middleware for metrics with OpenTelemetry.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryMetrics {
    request_count: Counter<u64>,
    error_count: Counter<u64>,
    duration: Histogram<f64>,
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
                .f64_histogram("poem_request_duration_ms")
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
    duration: Histogram<f64>,
    inner: E,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for OpenTelemetryMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let cx = Context::new();

        let mut labels = Vec::with_capacity(3);
        labels.push(trace::HTTP_METHOD.string(req.method().to_string()));
        labels.push(trace::HTTP_URL.string(req.original_uri().to_string()));

        let s = Instant::now();
        let res = self.inner.call(req).await.map(IntoResponse::into_response);
        let elapsed = s.elapsed();

        match &res {
            Ok(resp) => {
                if let Some(path_pattern) = resp.data::<PathPattern>() {
                    const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                    labels.push(HTTP_PATH_PATTERN.string(path_pattern.0.to_string()));
                }

                labels.push(trace::HTTP_STATUS_CODE.i64(resp.status().as_u16() as i64));
            }
            Err(err) => {
                if let Some(path_pattern) = err.data::<PathPattern>() {
                    const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                    labels.push(HTTP_PATH_PATTERN.string(path_pattern.0.to_string()));
                }

                labels.push(trace::HTTP_STATUS_CODE.i64(err.status().as_u16() as i64));
                self.error_count.add(&cx, 1, &labels);
                labels.push(trace::EXCEPTION_MESSAGE.string(err.to_string()));
            }
        }

        self.request_count.add(&cx, 1, &labels);
        self.duration
            .record(&cx, elapsed.as_secs_f64() * 1000.0, &labels);

        res
    }
}
