use libopentelemetry::{sdk::Resource, KeyValue};
use libprometheus::{Encoder, TextEncoder};

use crate::{
    http::{Method, StatusCode},
    Endpoint, IntoEndpoint, Request, Response,
};

/// An endpoint that exports metrics for Prometheus.
#[cfg_attr(docsrs, doc(cfg(feature = "prometheus")))]
#[derive(Default)]
pub struct PrometheusExporter {
    global_labels: Vec<KeyValue>,
}

impl PrometheusExporter {
    /// Create a `PrometheusExporter` endpoint.
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a global label.
    pub fn label(mut self, kv: KeyValue) -> Self {
        self.global_labels.push(kv);
        self
    }
}

impl IntoEndpoint for PrometheusExporter {
    type Endpoint = PrometheusExporterEndpoint;

    fn into_endpoint(self) -> Self::Endpoint {
        PrometheusExporterEndpoint {
            exporter: opentelemetry_prometheus::exporter()
                .with_resource(Resource::new(self.global_labels))
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

    async fn call(&self, req: Request) -> Self::Output {
        if req.method() != Method::GET {
            return StatusCode::METHOD_NOT_ALLOWED.into();
        }

        let encoder = TextEncoder::new();
        let metric_families = self.exporter.registry().gather();
        let mut result = Vec::new();
        match encoder.encode(&metric_families, &mut result) {
            Ok(()) => Response::builder().content_type("text/plain").body(result),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into(),
        }
    }
}
