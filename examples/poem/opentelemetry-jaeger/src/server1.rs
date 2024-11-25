use std::str::FromStr;

use opentelemetry::{
    global,
    trace::{FutureExt, SpanKind, TraceContextExt, Tracer as _, TracerProvider as _},
    Context, KeyValue,
};
use opentelemetry_http::HeaderInjector;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{Config, Tracer, TracerProvider},
    Resource,
};
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{OpenTelemetryMetrics, OpenTelemetryTracing},
    web::Data,
    EndpointExt, Route, Server,
};
use reqwest::{Client, Url};

fn init_tracer() -> TracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    opentelemetry_sdk::trace::TracerProvider::builder()
        .with_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "server1",
            )])),
        )
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()
                .expect("Trace exporter should initialize."),
            opentelemetry_sdk::runtime::Tokio,
        )
        .build()
}

#[handler]
async fn index(tracer: Data<&Tracer>, body: String) -> String {
    let mut span = tracer
        .span_builder("request/server2")
        .with_kind(SpanKind::Client)
        .start(tracer.0);
    let cx = Context::current_with_span(span);
    let client = Client::new();

    let req = {
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            Url::from_str("http://localhost:3002/api2").unwrap(),
        );
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut()))
        });
        *req.body_mut() = Some((body + "server1\n").into());
        req
    };

    let fut = async move {
        let cx = Context::current();
        let span = cx.span();

        span.add_event("Send request to server2".to_string(), vec![]);
        let resp = client.execute(req).await.unwrap();
        span.add_event(
            "Got response from server2!".to_string(),
            vec![KeyValue::new("status", resp.status().to_string())],
        );
        resp
    }
    .with_context(cx);

    fut.await.text().await.unwrap()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let tracer_provider = init_tracer();
    let tracer = tracer_provider.tracer("server1");

    let app = Route::new()
        .at("/api1", get(index))
        .data(tracer.clone())
        .with(OpenTelemetryMetrics::new())
        .with(OpenTelemetryTracing::new(tracer));

    Server::new(TcpListener::bind("0.0.0.0:3001"))
        .run(app)
        .await
}
