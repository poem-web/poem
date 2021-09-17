use poem::{handler, listener::TcpListener, route, route::post, web::Json, Server};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreateSomething {
    name: String,
}

#[handler]
fn hello(req: Json<CreateSomething>) -> Json<serde_json::Value> {
    Json(serde_json::json! ({
        "code": 0,
        "message": req.name,
    }))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = route().at("/hello", post(hello));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
