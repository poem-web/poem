use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{OpenTelemetryMetrics, OpenTelemetryTracing},
    EndpointExt, Route, Server,
};

fn init_tracer() -> SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("server2").build())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()
                .expect("Trace exporter should initialize."),
        )
        .build()
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
