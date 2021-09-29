use poem::{
    handler,
    lambda::{run, Error},
    route,
    route::get,
    web::Path,
};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = route().at("/hello/:name", get(hello));
    run(app).await
}
