use poem::{handler, listener::TcpListener, post, web::Json, Route, Server};
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
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/hello", post(hello));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
