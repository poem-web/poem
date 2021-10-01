use poem::{
    handler,
    listener::TcpListener,
    route,
    route::{get, Route},
    Server,
};

#[handler]
fn hello() -> String {
    "hello".to_string()
}

fn api() -> Route {
    route().at("/hello", get(hello))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug")
    }
    tracing_subscriber::fmt::init();

    let app = route().nest("/api", api());
    let server = Server::new(TcpListener::bind("127.0.0.1:3000")).await?;
    server.run(app).await
}
