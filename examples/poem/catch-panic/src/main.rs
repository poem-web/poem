use poem::{
    EndpointExt, Route, Server, handler,
    listener::TcpListener,
    middleware::{CatchPanic, Tracing},
};

#[handler]
fn index() {
    panic!("error!")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Route::new()
        .at("/", index)
        .with(Tracing)
        .with(CatchPanic::new());
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .name("hello-world")
        .run(app)
        .await
}
