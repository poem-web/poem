use poem::{
    ctx, get, handler,
    i18n::I18NResources,
    listener::TcpListener,
    tera::{filters, Tera, TeraTemplate, TeraTemplating},
    web::Path,
    EndpointExt, Route, Server,
};

#[handler]
fn index(tera: Tera) -> TeraTemplate {
    tera.render("index.html.tera", &ctx! {})
}

#[handler]
fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
    tera.render("hello.html.tera", &ctx! { "name": &name })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let resources = I18NResources::builder()
        .add_path("resources")
        .build()
        .unwrap();

    let app = Route::new()
        .at("/", get(index))
        .at("/hello/:name", get(hello))
        .with(TeraTemplating::from_glob("templates/**/*"))
        .using(filters::i18n::translate)
        .data(resources);

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
