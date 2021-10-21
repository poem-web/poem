use poem::{handler, listener::TcpListener, post, web::Json, Route, Server};
use serde::Deserialize;

/// Request entity
///
/// Defined as a DTO which need to implement [`serde::Deserialize`] which would
/// be deserialized from request body
#[derive(Debug, Deserialize)]
struct CreateSomething {
    name: String,
}

/// Real handle method for restful api.
///
/// `#[handler]`([`handler`]) macro marks the following method can be used for
/// handle api request.
///
/// `req: Json<CreateSomething>` in input parameter would be the data schema
/// of request. The fields in request body can be more than we defined in data
/// structure, but the ones other than we defined in structure would be ignored
#[handler]
fn hello(req: Json<CreateSomething>) -> Json<serde_json::Value> {
    Json(serde_json::json! ({
        "code": 0,
        "message": req.name,
    }))
}

/// Main method in service.
///
/// Details ref the doc in hello-world
///
/// usage:
/// 1. build & start the main.rs
/// 2. post a json data '{ "name": "$name" }' to the url: `http://localhost:3000/hello`
/// 3. '{ "code": 0, "message": "$name" }' will be returned
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/hello", post(hello));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
