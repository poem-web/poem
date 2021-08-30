use poem::{handler, route, route::Route, Server};

#[handler]
fn hello() -> String {
    format!("hello")
}

fn api() -> Route {
    let mut route = route();
    route.at("/hello").get(hello);
    route
}

#[tokio::main]
async fn main() {
    let mut app = route();
    app.nest("/api", api());
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
