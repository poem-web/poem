use poem::{get, handler, listener::TcpListener, web::Path, web::Html, Route, Server};
use poem::error::InternalServerError;
use tera::Tera;
use tera::Context;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera
    };
}

#[handler]
fn hello(Path(name): Path<String>) -> Result<Html<String>, poem::Error> {
    let mut context = Context::new();
    context.insert("name", &name);
    TEMPLATES
    .render("index.html.tera", &context)
    .map_err(InternalServerError)
    .map(Html)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new().at("/hello/:name", get(hello));
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}