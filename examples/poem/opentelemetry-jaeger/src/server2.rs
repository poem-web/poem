use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{Config, TracerProvider},
    Resource,
};
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{OpenTelemetryMetrics, OpenTelemetryTracing},
    EndpointExt, Route, Server,
};

fn init_tracer() -> TracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "server2",
            )])),
        )
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Trace Pipeline should initialize.")
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

    let tracer_provider = init_tracer();
    let tracer = tracer_provider.tracer("server2");

    let app = Route::new()
        .at("/api2", get(index))
        .data(tracer.clone())
        .with(OpenTelemetryMetrics::new())
        .with(OpenTelemetryTracing::new(tracer));

    Server::new(TcpListener::bind("0.0.0.0:3002"))
        .run(app)
        .await
}
