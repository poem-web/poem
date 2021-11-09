use poem::{
    get, handler, listener::TcpListener, Endpoint, EndpointExt, IntoResponse, Request, Response,
    Route, Server,
};

#[handler]
fn index() -> String {
    "hello".to_string()
}

async fn log<E: Endpoint>(next: E, req: Request) -> Response {
    println!("request: {}", req.uri().path());
    let resp = next.call(req).await.into_response();
    if resp.status().is_success() {
        println!("response: {}", resp.status());
    } else {
        println!("error: {}", resp.status());
    }
    resp
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).around(log);
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
