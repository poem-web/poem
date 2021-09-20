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
    let app = route().at("/", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000")
        .combine(TcpListener::bind("127.0.0.1:3001"))
        .combine(TcpListener::bind("127.0.0.1:3002"));
    let server = Server::new(listener).await?;
    server.run(app).await
}
