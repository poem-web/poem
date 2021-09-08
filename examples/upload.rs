use poem::{handler, route, route::post, web::Multipart, Server};

#[handler]
async fn index(mut multipart: Multipart) {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().map(ToString::to_string);
        let file_name = field.file_name().map(ToString::to_string);
        if let Ok(bytes) = field.bytes().await {
            println!(
                "name={:?} filename={:?} length={}",
                name,
                file_name,
                bytes.len()
            );
        }
    }
}

#[tokio::main]
async fn main() {
    let app = route().at("/", post(index));
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
