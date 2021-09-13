use poem::{
    handler, http::StatusCode, listener::TcpListener, route, route::get, web::Path, EndpointExt,
    Response, Server,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at(
        "/hello/:name",
        get(hello.after(|resp| async move {
            if resp.status() == StatusCode::NOT_FOUND {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")
            } else {
                resp
            }
        })),
    );

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
