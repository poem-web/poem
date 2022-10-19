use std::collections::HashMap;

use libopentelemetry::sdk::{
    export::metrics::aggregation,
    metrics::{controllers, controllers::BasicController, processors, selectors},
};
use libprometheus::{Encoder, Registry, TextEncoder};

use crate::{
    http::{Method, StatusCode},
    Endpoint, IntoEndpoint, Request, Response, Result,
};

/// An endpoint that exports metrics for Prometheus.
///
/// # Example
///
/// ```
/// use libopentelemetry::sdk::{
///     export::metrics::aggregation,
///     metrics::{controllers, processors, selectors},
/// };
/// use poem::{endpoint::PrometheusExporter, Route};
///
/// let controller = controllers::basic(
///     processors::factory(
///         selectors::simple::histogram([1.0, 2.0, 5.0, 10.0, 20.0, 50.0]),
///         aggregation::cumulative_temporality_selector(),
///     )
///     .with_memory(true),
/// )
/// .build();
///
/// let app = Route::new().nest("/metrics", PrometheusExporter::with_controller(controller));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "prometheus")))]
pub struct PrometheusExporter {
    controller: BasicController,
    prefix: Option<String>,
    labels: HashMap<String, String>,
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusExporter {
    /// Create a `PrometheusExporter` endpoint.
    pub fn new() -> Self {
        let controller = controllers::basic(
            processors::factory(
                selectors::simple::histogram([1.0, 2.0, 5.0, 10.0, 20.0, 50.0]),
                aggregation::cumulative_temporality_selector(),
            )
            .with_memory(true),
        )
        .build();

        Self {
            controller,
            prefix: None,
            labels: HashMap::new(),
        }
    }

    /// Create a `PrometheusExporter` endpoint with a controller.
    pub fn with_controller(controller: BasicController) -> Self {
        Self {
            controller,
            prefix: None,
            labels: HashMap::new(),
        }
    }

    /// Set a common namespace for all registered collectors.
    #[must_use]
    pub fn prefix(self, prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            ..self
        }
    }

    /// Add a common label for all registered collectors.
    #[must_use]
    pub fn label(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(name.into(), value.into());
        self
    }
}

impl IntoEndpoint for PrometheusExporter {
    type Endpoint = PrometheusExporterEndpoint;

    fn into_endpoint(self) -> Self::Endpoint {
        PrometheusExporterEndpoint {
            exporter: opentelemetry_prometheus::exporter(self.controller)
                .with_registry(
                    Registry::new_custom(self.prefix, Some(self.labels))
                        .expect("create prometheus registry"),
                )
                .init(),
        }
    }
}

#[doc(hidden)]
pub struct PrometheusExporterEndpoint {
    exporter: opentelemetry_prometheus::PrometheusExporter,
}

#[async_trait::async_trait]
impl Endpoint for PrometheusExporterEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() != Method::GET {
            return Ok(StatusCode::METHOD_NOT_ALLOWED.into());
        }

        let encoder = TextEncoder::new();
        let metric_families = self.exporter.registry().gather();
        let mut result = Vec::new();
        match encoder.encode(&metric_families, &mut result) {
            Ok(()) => Ok(Response::builder().content_type("text/plain").body(result)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR.into()),
        }
    }
}
