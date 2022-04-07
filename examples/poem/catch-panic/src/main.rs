use poem::{
    handler,
    listener::TcpListener,
    middleware::{CatchPanic, Tracing},
    EndpointExt, Route, Server,
};

#[handler]
fn index() {
    panic!("error!")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/", index)
        .with(Tracing)
        .with(CatchPanic::new());
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .name("hello-world")
        .run(app)
        .await
}
