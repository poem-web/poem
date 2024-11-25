use std::time::Instant;

use libopentelemetry::{
    global,
    metrics::{Counter, Histogram},
    Key, KeyValue,
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
                .build(),
            error_count: meter
                .u64_counter("poem_errors_count")
                .with_description("failed request count (since start of service)")
                .build(),
            duration: meter
                .f64_histogram("poem_request_duration_ms")
                .with_unit("milliseconds")
                .with_description(
                    "request duration histogram (in milliseconds, since start of service)",
                )
                .build(),
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

/// Endpoint for the OpenTelemetryMetrics middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryMetricsEndpoint<E> {
    request_count: Counter<u64>,
    error_count: Counter<u64>,
    duration: Histogram<f64>,
    inner: E,
}

impl<E: Endpoint> Endpoint for OpenTelemetryMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut labels = Vec::with_capacity(3);
        labels.push(KeyValue::new(
            trace::HTTP_REQUEST_METHOD,
            req.method().to_string(),
        ));
        labels.push(KeyValue::new(
            trace::URL_FULL,
            req.original_uri().to_string(),
        ));

        let s = Instant::now();
        let res = self.inner.call(req).await.map(IntoResponse::into_response);
        let elapsed = s.elapsed();

        match &res {
            Ok(resp) => {
                if let Some(path_pattern) = resp.data::<PathPattern>() {
                    const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                    labels.push(KeyValue::new(HTTP_PATH_PATTERN, path_pattern.0.to_string()));
                }

                labels.push(KeyValue::new(
                    trace::HTTP_RESPONSE_STATUS_CODE,
                    resp.status().as_u16() as i64,
                ));
            }
            Err(err) => {
                if let Some(path_pattern) = err.data::<PathPattern>() {
                    const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                    labels.push(KeyValue::new(HTTP_PATH_PATTERN, path_pattern.0.to_string()));
                }

                labels.push(KeyValue::new(
                    trace::HTTP_RESPONSE_STATUS_CODE,
                    err.status().as_u16() as i64,
                ));
                self.error_count.add(1, &labels);
                labels.push(KeyValue::new(trace::EXCEPTION_MESSAGE, err.to_string()));
            }
        }

        self.request_count.add(1, &labels);
        self.duration
            .record(elapsed.as_secs_f64() * 1000.0, &labels);

        res
    }
}
