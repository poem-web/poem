use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{ReqId, RequestId, ReuseId, Tracing},
    EndpointExt, Route,
};

#[handler]
fn show_request_id(id: ReqId) -> ReqId {
    id
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", get(show_request_id))
        .with(Tracing)
        // `RequestId` must be applied _after_ tracing, for the ID to be logged in the trace span
        .with(RequestId::default().reuse_id(ReuseId::Use));

    println!("example server listening on 127.0.0.1:8080");
    println!("try `curl -v http://127.0.0.1:8080/`");
    println!("try `curl -v -H 'x-request-id: 12345' http://127.0.0.1:8080/`");
    poem::Server::new(TcpListener::bind("127.0.0.1:8080"))
        .run(app)
        .await
}
