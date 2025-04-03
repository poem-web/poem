use libprometheus::{Encoder, Registry, TextEncoder};

use crate::{
    Endpoint, IntoEndpoint, Request, Response, Result,
    http::{Method, StatusCode},
};

/// An endpoint that exports metrics for Prometheus.
///
/// # Example
///
/// ```
/// use libprometheus::Registry;
/// use poem::{Route, endpoint::PrometheusExporter};
///
/// let registry = Registry::new();
/// let app = Route::new().nest("/metrics", PrometheusExporter::new(registry));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "prometheus")))]
pub struct PrometheusExporter {
    registry: Registry,
}

impl PrometheusExporter {
    /// Create a `PrometheusExporter` endpoint.
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }
}

impl IntoEndpoint for PrometheusExporter {
    type Endpoint = PrometheusExporterEndpoint;

    fn into_endpoint(self) -> Self::Endpoint {
        PrometheusExporterEndpoint {
            registry: self.registry.clone(),
        }
    }
}

#[doc(hidden)]
pub struct PrometheusExporterEndpoint {
    registry: Registry,
}

impl Endpoint for PrometheusExporterEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() != Method::GET {
            return Ok(StatusCode::METHOD_NOT_ALLOWED.into());
        }

        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut result = Vec::new();
        match encoder.encode(&metric_families, &mut result) {
            Ok(()) => Ok(Response::builder()
                .content_type(encoder.format_type())
                .body(result)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::TestClient;

    #[tokio::test]
    async fn test_content_type() {
        let client = TestClient::new(PrometheusExporter::new(Registry::new()));

        let resp = client.get("/metrics").send().await;

        resp.assert_status_is_ok();
        resp.assert_content_type("text/plain; version=0.0.4");
    }
}
