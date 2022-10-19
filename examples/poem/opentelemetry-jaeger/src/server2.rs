use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::Tracer},
};
use poem::{
    endpoint::PrometheusExporter,
    get, handler,
    listener::TcpListener,
    middleware::{OpenTelemetryMetrics, OpenTelemetryTracing},
    EndpointExt, Route, Server,
};

fn init_tracer() -> Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());
    opentelemetry_jaeger::new_collector_pipeline()
        .with_service_name("poem")
        .with_endpoint("http://localhost:14268/api/traces")
        .with_hyper()
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap()
}

#[handler]
fn index(body: String) -> String {
    body + "server2\n"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let tracer = init_tracer();

    let app = Route::new()
        .at("/api2", get(index))
        .at("/metrics", PrometheusExporter::new())
        .data(tracer.clone())
        .with(OpenTelemetryMetrics::new())
        .with(OpenTelemetryTracing::new(tracer));

    Server::new(TcpListener::bind("127.0.0.1:3002"))
        .run(app)
        .await
}
