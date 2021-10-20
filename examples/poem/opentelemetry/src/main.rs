use opentelemetry::{
    global,
    sdk::{
        export::trace::stdout,
        propagation::TraceContextPropagator,
        trace::{self, Sampler},
    },
    trace::Tracer,
};
use poem::{
    endpoint::PrometheusExporter,
    get, handler,
    listener::TcpListener,
    middleware::{OpenTelemetryMetrics, OpenTelemetryTracing},
    EndpointExt, Route, Server,
};

fn init_tracer() -> impl Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());
    stdout::new_pipeline()
        .with_trace_config(trace::config().with_sampler(Sampler::AlwaysOn))
        .install_simple()
}

#[handler]
fn index() -> &'static str {
    "hello world"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", get(index))
        .at("/metrics", PrometheusExporter::new())
        .with(OpenTelemetryMetrics::new())
        .with(OpenTelemetryTracing::new(init_tracer()));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
