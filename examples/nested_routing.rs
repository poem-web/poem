use poem::{
    handler,
    listener::TcpListener,
    route,
    route::{get, Route},
    Server,
};

#[handler]
fn hello() -> String {
    format!("hello")
}

fn api() -> Route {
    route().at("/hello", get(hello))
}

#[tokio::main]
async fn main() {
    let app = route().nest("/api", api());
    let server = Server::new(TcpListener::bind("127.0.0.1:3000"))
        .await
        .unwrap();
    server.run(app).await.unwrap();
}
