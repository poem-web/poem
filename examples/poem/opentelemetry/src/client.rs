use std::str::FromStr;

use opentelemetry::{
    global,
    sdk::{
        export::trace::stdout,
        propagation::TraceContextPropagator,
        trace::{self, Sampler},
    },
    trace::{TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_http::HeaderInjector;
use reqwest::{Client, Method, Url};

fn init_tracer() -> impl Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());
    stdout::new_pipeline()
        .with_trace_config(trace::config().with_sampler(Sampler::AlwaysOn))
        .install_simple()
}

#[tokio::main]
async fn main() {
    let _tracer = init_tracer();
    let client = Client::new();
    let span = global::tracer("example-opentelemetry/client").start("request");
    let cx = Context::current_with_span(span);

    let mut req =
        reqwest::Request::new(Method::GET, Url::from_str("http://localhost:3000").unwrap());
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut HeaderInjector(&mut req.headers_mut()))
    });
    *req.body_mut() = Some(vec![].into());

    let resp = client.execute(req).await.unwrap();

    cx.span().add_event(
        "Got response!".to_string(),
        vec![KeyValue::new("status", resp.status().to_string())],
    );
}
