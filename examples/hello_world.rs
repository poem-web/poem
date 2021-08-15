use poem::{prelude::*, web::Path};

async fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() {
    let app = route().at("/hello/:name", get(hello));
    serve(app).run("127.0.0.1:3000").await.unwrap();
}
