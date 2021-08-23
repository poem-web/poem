use derive_more::Display;
use poem::{handler, http::StatusCode, route, Error, Response, ResponseError, Result, Server};

#[derive(Debug, Display)]
struct CustomError;

impl ResponseError for CustomError {
    fn as_response(&self) -> Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("CustomError")
    }
}

#[handler(method = "get")]
fn index() -> Result<String> {
    Err(Error::new(CustomError))
}

#[tokio::main]
async fn main() {
    let app = route().at("/", index);
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
