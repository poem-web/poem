use poem::{get, handler, web::Path, Route};
use poem_lambda::{run, Error};

/// Real handle method for restful api.
///
/// Details ref the doc in hello-world
#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

/// Main method in service.
///
/// [`poem_lambda`] was designed to run a lambda server/operator in Amazon
/// Lambda service [`Route`], [`Route::at`] and other stuffs' attributes are the
/// same struct [`poem_lambda::run`] would take the responsible of holding the
/// service, monitoring port, taking action on request .etc
///
/// usage:
/// 1. build & start the main.rs
/// 2. deploy as an Amazon lambda service
/// 3. curl the url: `http://localhost:3000/hello/$name`
/// 4. "hello $name" will be returned
#[tokio::main]
async fn main() -> Result<(), Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/hello/:name", get(hello));
    run(app).await
}
