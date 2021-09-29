use poem::{handler, route, route::get, web::Path};
use poem_lambda::{run, Error};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = route().at("/hello/:name", get(hello));
    run(app).await
}
