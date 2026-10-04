use once_cell::sync::Lazy;
use poem::{
    Route, Server,
    error::InternalServerError,
    get, handler,
    listener::TcpListener,
    web::{Html, Path},
};
use tera::{Context, Tera};

fn load_templates(glob: &str) -> tera::TeraResult<Tera> {
    let mut tera = Tera::new();
    tera.load_from_glob(glob)?;
    tera.autoescape_on([".html", ".html.tera", ".sql"]);
    Ok(tera)
}

static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    load_templates("templates/**/*").unwrap_or_else(|e| {
        println!("Parsing error(s): {e}");
        ::std::process::exit(1);
    })
});

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
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_and_escapes_html_template() {
        let tera = load_templates(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();
        let mut context = Context::new();
        context.insert("name", "<script>alert('hi')</script>");

        let rendered = tera.render("index.html.tera", &context).unwrap();
        assert!(rendered.starts_with("<h1>Hello &lt;script&gt;"));
        assert!(!rendered.contains("<script>"));
    }
}
