use poem::{listener::TcpListener, route, service::Files, Server};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = route().nest(
        "/",
        Files::new("./examples/static-files/files").show_files_listing(),
    );
    let server = Server::new(TcpListener::bind("127.0.0.1:3000")).await?;
    server.run(app).await
}
