use poem::{get, http::StatusCode, route, web::Path, EndpointExt, Response, Server};

#[get]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route()
        .at("/hello/:name", hello)
        .map_to_response()
        .map(|res| async move {
            match res {
                Ok(resp) if resp.status() == StatusCode::NOT_FOUND => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")),
                Ok(resp) => Ok(resp),
                Err(_) => unreachable!(),
            }
        });
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
