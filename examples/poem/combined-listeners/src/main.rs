use poem::{
    handler,
    listener::{Listener, TcpListener},
    route,
    route::get,
    IntoResponse, Server,
};

#[handler]
fn hello() -> impl IntoResponse {
    "hello"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug")
    }
    tracing_subscriber::fmt::init();

    let app = route().at("/", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000")
        .combine(TcpListener::bind("127.0.0.1:3001"))
        .combine(TcpListener::bind("127.0.0.1:3002"));
    let server = Server::new(listener).await?;
    server.run(app).await
}
