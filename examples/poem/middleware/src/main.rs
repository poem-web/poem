use poem::{
    async_trait, get, handler, listener::TcpListener, Endpoint, EndpointExt, IntoResponse,
    Middleware, Request, Response, Route, Server,
};

/// Middleware struct definition.
struct Log;

/// [`Log`] and its impl block defines the [`Middleware`] structure.
///
/// Output type of it should be the the implement of [`Endpoint`] which was used
/// for handle the input data ([`Request`] in this sample).
/// The transform function convert the input [`Endpoint`] into the "real"
/// endpoint which maintain the real handle logic.
impl<E: Endpoint> Middleware<E> for Log {
    type Output = LogImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        LogImpl(ep)
    }
}

/// "Real" endpoint struct which maintain the real handle logic.
struct LogImpl<E>(E);

/// "Real" endpoint struct implement
///
/// `#[async_trait]` ([`async_trait`]) macro marked the logic can be executed
/// asynchronous Output type should be the same with the original one in biz
/// logic ([`Response`] in this sample as it would be the response against the
/// api) Function `call` is the "real" logic to handle the input parameters
/// `self.0.call(req).await.into_response()` was the original biz logic.
/// By adding extra logic before & after the original one to extend the
/// attribute or add commonly logics [`poem`] also have the [`Endpoint::before`]
/// and [`Endpoint::after`] function instead the `with` to specifically take
/// action on each step of biz logic's invoking
#[async_trait]
impl<E: Endpoint> Endpoint for LogImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Self::Output {
        println!("request: {}", req.uri().path());
        let resp = self.0.call(req).await.into_response();
        if resp.status().is_success() {
            println!("response: {}", resp.status());
        } else {
            println!("error: {}", resp.status());
        }
        resp
    }
}

/// Real handle method for restful api.
///
/// Details ref the doc in hello-world
#[handler]
fn index() -> String {
    "hello".to_string()
}

/// Main method in service.
///
/// `let app = Route::new().at("/", get(index)).with(Log);` has extra code
/// `with($middleware)` which used for assigning the aspect of biz logic
/// [`Endpoint::with`] will do some extra logic (defined in the implement of
/// struct `Log`) as an aspect around the original biz logic(defined in function
/// `index`). Other details ref the doc in hello-world
///
/// usage:
/// 1. build & start the main.rs
/// 2. curl the url: `http://localhost:3000`
/// 3. "hello" will be returned
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).with(Log);
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
