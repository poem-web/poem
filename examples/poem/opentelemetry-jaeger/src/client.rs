use std::str::FromStr;

use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::Tracer},
    trace::{FutureExt, TraceContextExt, Tracer as _},
    Context, KeyValue,
};
use opentelemetry_http::HeaderInjector;
use reqwest::{Client, Method, Url};

fn init_tracer() -> Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());
    opentelemetry_jaeger::new_collector_pipeline()
        .with_service_name("poem")
        .with_endpoint("http://localhost:14268/api/traces")
        .with_hyper()
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap()
}

#[tokio::main]
async fn main() {
    let _tracer = init_tracer();
    let client = Client::new();
    let span = global::tracer("example-opentelemetry/client").start("request/server1");
    let cx = Context::current_with_span(span);

    let req = {
        let mut req = reqwest::Request::new(
            Method::GET,
            Url::from_str("http://localhost:3001/api1").unwrap(),
        );
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut()));
            println!("{:?}", req.headers_mut());
        });
        *req.body_mut() = Some("client\n".into());
        req
    };

    async move {
        let cx = Context::current();
        let span = cx.span();

        span.add_event("Send request to server1".to_string(), vec![]);
        let resp = client.execute(req).await.unwrap();
        span.add_event(
            "Got response from server1!".to_string(),
            vec![KeyValue::new("status", resp.status().to_string())],
        );
        println!("{}", resp.text().await.unwrap());
    }
    .with_context(cx)
    .await;

    global::shutdown_tracer_provider();
}
