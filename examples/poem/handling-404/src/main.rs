use poem::{
    get, handler, http::StatusCode, listener::TcpListener, web::Path, EndpointExt, Response, Route,
    Server,
};

/// Real handle method for restful api.
///
/// Details ref the doc in hello-world
#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

/// Main method in service.
///
/// ```
/// let app = Route::new()
///     .at("/hello/:name", get(hello))
///     .after(|resp| async move {
///         if resp.status() == StatusCode::NOT_FOUND {
///             Response::builder()
///                 .status(StatusCode::NOT_FOUND)
///                 .body("haha")
///         } else {
///             resp
///         }
///     });
/// ```
/// register a common handler for the UNKNOWN apis other than sample one (GET
/// /hello/$name in this sample)
///
/// usage:
/// 1. build & start the main.rs
/// 2. curl the url: `http://localhost:3000/hello/$name`
/// 3. "hello $name" with normal status will be returned
/// 4. curl the url: `http://localhost:3000/hello` || url: `http://localhost:3000/hello/$name/123`
/// 5. "haha" with
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new()
        .at("/hello/:name", get(hello))
        .after(|resp| async move {
            if resp.status() == StatusCode::NOT_FOUND {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("haha")
            } else {
                resp
            }
        });

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
