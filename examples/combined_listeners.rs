use poem::{
    handler,
    listener::{IntoAcceptor, TcpListener},
    route,
    route::get,
    web::Path,
    Server,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000")
        .combine(TcpListener::bind("127.0.0.1:3001"))
        .combine(TcpListener::bind("127.0.0.1:3002"));
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
