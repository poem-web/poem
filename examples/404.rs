use poem::{handler, http::StatusCode, route, web::Path, EndpointExt, Response, Server};

#[handler(method = "get")]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", hello).after(|resp| async move {
        if resp.status() == StatusCode::NOT_FOUND {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("haha")
        } else {
            resp
        }
    });
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
