use poem::{
    error::InternalServerError,
    get, handler,
    listener::TcpListener,
    web::{Html, Path},
    Route, Server,
    EndpointExt,
    tera::TeraTemplating
};
use tera::{Context, Tera};

#[handler]
fn hello(Path(name): Path<String>, tera: Tera) -> Result<Html<String>, poem::Error> {
    let mut context = Context::new();
    context.insert("name", &name);
    tera
        .render("index.html.tera", &context)
        .map_err(InternalServerError)
        .map(Html)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .with(TeraTemplating::from_glob("templates/**/*"));

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
