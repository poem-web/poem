#[cfg(unix)]
#[tokio::main]
async fn main() {
    use poem::{
        handler, http::Uri, listener::UnixListener, route, route::get, IntoResponse, Server,
    };

    #[handler]
    fn hello(uri: &Uri) -> impl IntoResponse {
        uri.path().to_string()
    }

    let app = route().at("/", get(hello));
    let listener = UnixListener::bind("./unix-socket");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}

#[cfg(not(unix))]
fn main() {
    println!("This example works only on Unix systems!");
}
