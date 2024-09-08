use std::sync::Arc;

use libopentelemetry::{
    global,
    trace::{FutureExt, Span, SpanKind, TraceContextExt, Tracer},
    Context, Key, KeyValue,
};
use opentelemetry_http::HeaderExtractor;
use opentelemetry_semantic_conventions::{attribute, resource};

use crate::{
    route::PathPattern,
    web::{headers::HeaderMapExt, RealIp},
    Endpoint, FromRequest, IntoResponse, Middleware, Request, Response, Result,
};

/// Middleware for tracing with OpenTelemetry.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryTracing<T> {
    tracer: Arc<T>,
}

impl<T> OpenTelemetryTracing<T> {
    /// Create `OpenTelemetryTracing` middleware with `tracer`.
    pub fn new(tracer: T) -> Self {
        Self {
            tracer: Arc::new(tracer),
        }
    }
}

impl<T, E> Middleware<E> for OpenTelemetryTracing<T>
where
    T: Tracer + Send + Sync,
    T::Span: Send + Sync + 'static,
    E: Endpoint,
{
    type Output = OpenTelemetryTracingEndpoint<T, E>;

    fn transform(&self, ep: E) -> Self::Output {
        OpenTelemetryTracingEndpoint {
            tracer: self.tracer.clone(),
            inner: ep,
        }
    }
}

/// Endpoint for the `OpenTelemetryTracing` middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
pub struct OpenTelemetryTracingEndpoint<T, E> {
    tracer: Arc<T>,
    inner: E,
}

impl<T, E> Endpoint for OpenTelemetryTracingEndpoint<T, E>
where
    T: Tracer + Send + Sync,
    T::Span: Send + Sync + 'static,
    E: Endpoint,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let remote_addr = RealIp::from_request_without_body(&req)
            .await
            .ok()
            .and_then(|real_ip| real_ip.0)
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| req.remote_addr().to_string());

        let parent_cx = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(req.headers()))
        });

        let mut attributes = Vec::new();
        attributes.push(KeyValue::new(
            resource::TELEMETRY_SDK_NAME,
            env!("CARGO_CRATE_NAME"),
        ));
        attributes.push(KeyValue::new(
            resource::TELEMETRY_SDK_VERSION,
            env!("CARGO_PKG_VERSION"),
        ));
        attributes.push(KeyValue::new(resource::TELEMETRY_SDK_LANGUAGE, "rust"));
        attributes.push(KeyValue::new(
            attribute::HTTP_REQUEST_METHOD,
            req.method().to_string(),
        ));
        attributes.push(KeyValue::new(
            attribute::URL_FULL,
            req.original_uri().to_string(),
        ));
        attributes.push(KeyValue::new(attribute::CLIENT_ADDRESS, remote_addr));
        attributes.push(KeyValue::new(
            attribute::NETWORK_PROTOCOL_VERSION,
            format!("{:?}", req.version()),
        ));

        let method = req.method().to_string();
        let mut span = self
            .tracer
            .span_builder(format!("{} {}", method, req.uri()))
            .with_kind(SpanKind::Server)
            .with_attributes(attributes)
            .start_with_context(&*self.tracer, &parent_cx);

        span.add_event("request.started".to_string(), vec![]);

        async move {
            let res = self.inner.call(req).await;
            let cx = Context::current();
            let span = cx.span();

            match res {
                Ok(resp) => {
                    let resp = resp.into_response();

                    if let Some(path_pattern) = resp.data::<PathPattern>() {
                        const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                        span.update_name(format!("{} {}", method, path_pattern.0));
                        span.set_attribute(KeyValue::new(
                            HTTP_PATH_PATTERN,
                            path_pattern.0.to_string(),
                        ));
                    }

                    span.add_event("request.completed".to_string(), vec![]);
                    span.set_attribute(KeyValue::new(
                        attribute::HTTP_RESPONSE_STATUS_CODE,
                        resp.status().as_u16() as i64,
                    ));
                    if let Some(content_length) =
                        resp.headers().typed_get::<headers::ContentLength>()
                    {
                        span.set_attribute(KeyValue::new(
                            attribute::HTTP_RESPONSE_BODY_SIZE,
                            content_length.0 as i64,
                        ));
                    }
                    Ok(resp)
                }
                Err(err) => {
                    if let Some(path_pattern) = err.data::<PathPattern>() {
                        const HTTP_PATH_PATTERN: Key = Key::from_static_str("http.path_pattern");
                        span.update_name(format!("{} {}", method, path_pattern.0));
                        span.set_attribute(KeyValue::new(
                            HTTP_PATH_PATTERN,
                            path_pattern.0.to_string(),
                        ));
                    }

                    span.set_attribute(KeyValue::new(
                        attribute::HTTP_RESPONSE_STATUS_CODE,
                        err.status().as_u16() as i64,
                    ));
                    span.add_event(
                        "request.error".to_string(),
                        vec![KeyValue::new(attribute::EXCEPTION_MESSAGE, err.to_string())],
                    );
                    Err(err)
                }
            }
        }
        .with_context(Context::current_with_span(span))
        .await
    }
}
