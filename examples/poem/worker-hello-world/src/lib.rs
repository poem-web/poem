use poem::{get, handler, web::Path, Route};
use poem_worker::{CloudflareProperties, Server};
use worker::event;

#[handler]
fn hello(Path(name): Path<String>, _cf: CloudflareProperties) -> String {
    format!("hello: {}", name)
}

#[event(start)]
fn start() {
    let app = Route::new().at("/hello/:name", get(hello));

    Server::new().run(app);
}
