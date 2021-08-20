use derive_more::Display;
use poem::{get, handler, http::StatusCode, route, Response, ResponseError, Server};

#[derive(Debug, Display)]
struct CustomError;

impl ResponseError for CustomError {
    fn as_response(&self) -> Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("CustomError")
    }
}

#[handler]
fn index() -> Result<String, CustomError> {
    Err(CustomError)
}

#[tokio::main]
async fn main() {
    let app = route().at("/", get(index));
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
