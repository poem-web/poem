use poem::{get, route, route::Route, Server};

#[get]
fn hello() -> String {
    format!("hello")
}

fn api() -> Route {
    let route = route().at("/hello", hello);
    route
}

#[tokio::main]
async fn main() {
    let app = route().nest("/api", api());
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
