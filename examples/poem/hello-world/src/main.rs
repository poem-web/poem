use poem::{get, handler, listener::TcpListener, web::Path, Route, Server};

/// Real handle method for restful api.
///
/// `#[handler]`([`handler`]) macro marks the following method can be used for
/// handle api request.
///
/// Struct [`Path`] in param part is used for getting the path value and bind
/// the variable defined in `Path<name>`.
///
/// The path value can support any data structure which implemented [`Eq`],
/// [`PartialEq`], [`Copy`], string and numeric ones were in recommended.
///
/// Returned data will be set as the response against api's request if no more
/// configuration or other packaging process.
#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

/// Main method in service.
///
/// [`tokio::main`] was in necessary as poem was depend on it.
///
/// [`tracing_subscriber`] was an optional component which was used for
/// collecting the tracing labels.
///
/// [`Route`] hold the routers in service, the definition would be like:
/// `Route::new().at("$path", $http_method($handler_method)); =>
/// Route::new().at("/hello/:name", get(hello));`
///
/// Function [`Route::at`] is used for defined the end point of api, and bind
/// the handler method against the api. The variable name (`:name` in this
/// sample) must be the same with the key defined by [`Path(T)`] in input
/// parameter of handler method (`Path(name)` in this sample)
///
/// The `$http_method` ([`get`] in this sample) defines the accept route method
/// of the api.
///
/// The `$handler_method` ([`hello`] in this sample) point out the method which
/// would handle request for this api.
///
/// The other parts, maintain the server and tcp listener were the same with the
/// sample code in <The Book>.
///
/// usage:
/// 1. build & start the main.rs
/// 2. curl the url: `http://localhost:3000/hello/$name`
/// 3. "hello $name" will be returned
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/hello/:name", get(hello));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
