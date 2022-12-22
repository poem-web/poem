//! Poem is a full-featured and easy-to-use web framework with the Rust
//! programming language.
//!
//! # Table of contents
//!
//! - [Quickstart](#quickstart)
//! - [Endpoint](#endpoint)
//! - [Extractors](#extractors)
//! - [Routing](#routing)
//! - [Responses](#responses)
//! - [Handling errors](#handling-errors)
//! - [Middleware](#middleware)
//! - [Crate features](#crate-features)
//!
//! # Quickstart
//!
//! ```no_run
//! use poem::{get, handler, listener::TcpListener, web::Path, IntoResponse, Route, Server};
//!
//! #[handler]
//! fn hello(Path(name): Path<String>) -> String {
//!     format!("hello: {}", name)
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), std::io::Error> {
//!     let app = Route::new().at("/hello/:name", get(hello));
//!     Server::new(TcpListener::bind("127.0.0.1:3000"))
//!         .run(app)
//!         .await
//! }
//! ```
//!
//! # Endpoint
//!
//! The [`Endpoint`] trait represents a type that can handle HTTP requests, and
//! it returns the `Result<T: IntoResponse, Error>` type.
//!
//! The [`handler`] macro is used to convert a function into an endpoint.
//!
//! ```
//! use poem::{
//!     error::NotFoundError, handler, http::StatusCode, test::TestClient, Endpoint, Request,
//!     Result,
//! };
//!
//! #[handler]
//! fn return_str() -> &'static str {
//!     "hello"
//! }
//!
//! #[handler]
//! fn return_err() -> Result<&'static str, NotFoundError> {
//!     Err(NotFoundError)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let resp = TestClient::new(return_str).get("/").send().await;
//! resp.assert_status_is_ok();
//! resp.assert_text("hello").await;
//!
//! let resp = TestClient::new(return_err).get("/").send().await;
//! resp.assert_status(StatusCode::NOT_FOUND);
//! # });
//! ```
//!
//! # Extractors
//!
//! The extractor is used to extract something from the HTTP request.
//!
//! `Poem` provides some [commonly used extractors](web::FromRequest) for
//! extracting something from HTTP requests.
//!
//! In the following example, the `index` function uses 3 extractors to extract
//! the remote address, HTTP method and URI.
//!
//! ```
//! use poem::{
//!     handler,
//!     http::{Method, Uri},
//!     web::RemoteAddr,
//! };
//!
//! #[handler]
//! fn index(remote_addr: &RemoteAddr, method: Method, uri: &Uri) {}
//! ```
//!
//! By default, the extractor will return a `400 Bad Request` when an error
//! occurs, but sometimes you may want to change this behavior, so you can
//! handle the error yourself.
//!
//! In the following example, when the [`Query`](web::Query) extractor fails, it
//! will return a `500 Internal Server` response and the reason for the error.
//!
//! ```
//! use poem::{
//!     error::ParseQueryError, handler, http::StatusCode, web::Query, IntoResponse, Response,
//!     Result,
//! };
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize)]
//! struct Params {
//!     name: String,
//! }
//!
//! #[handler]
//! fn index(res: Result<Query<Params>>) -> Result<impl IntoResponse> {
//!     match res {
//!         Ok(Query(params)) => Ok(params.name.into_response()),
//!         Err(err) if err.is::<ParseQueryError>() => Ok(Response::builder()
//!             .status(StatusCode::INTERNAL_SERVER_ERROR)
//!             .body(err.to_string())),
//!         Err(err) => Err(err),
//!     }
//! }
//! ```
//!
//! # Routing
//!
//! There are three available routes.
//!
//! - [`Route`] Routing for path
//! - [`RouteDomain`] Routing for domain
//! - [`RouteMethod`] Routing for HTTP method
//!
//! ```
//! use poem::{get, handler, post, web::Path, Route};
//!
//! #[handler]
//! async fn get_user(id: Path<String>) {}
//!
//! #[handler]
//! async fn delete_user(id: Path<String>) {}
//!
//! #[handler]
//! async fn create_user() {}
//!
//! let app = Route::new()
//!     .at("/user/:id", get(get_user).delete(delete_user))
//!     .at("/user", post(create_user));
//! ```
//!
//! You can create custom extractors, see also [`FromRequest`].
//!
//! # Responses
//!
//! All types that can be converted to HTTP response [`Response`] should
//! implement [`IntoResponse`].
//!
//! In the following example, the `string_response` and `status_response`
//! functions return the `String` and `StatusCode` types, because `Poem` has
//! implemented the [`IntoResponse`] trait for them.
//!
//! The `no_response` function does not return a value. We can think that
//! its return type is `()`, and `Poem` also implements [`IntoResponse`] for
//! `()`, which is always converted to `200 OK`.
//!
//! The `result_response` function returns a `Result` type, which means that an
//! error may occur.
//! ```
//! use poem::{handler, http::StatusCode, Result};
//!
//! #[handler]
//! fn string_response() -> String {
//!     todo!()
//! }
//!
//! #[handler]
//! fn status_response() -> StatusCode {
//!     todo!()
//! }
//!
//! #[handler]
//! fn no_response() {}
//!
//! #[handler]
//! fn result_response() -> Result<String> {
//!     todo!()
//! }
//! ```
//!
//! # Handling errors
//!
//! The following example returns customized content when
//! [`NotFoundError`](error::NotFoundError) occurs.
//!
//! ```
//! use poem::{
//!     error::NotFoundError, handler, http::StatusCode, EndpointExt, IntoResponse, Response, Route,
//! };
//!
//! #[handler]
//! fn foo() {}
//!
//! #[handler]
//! fn bar() {}
//!
//! let app =
//!     Route::new()
//!         .at("/foo", foo)
//!         .at("/bar", bar)
//!         .catch_error(|err: NotFoundError| async move {
//!             Response::builder()
//!                 .status(StatusCode::NOT_FOUND)
//!                 .body("custom not found")
//!         });
//! ```
//!
//! # Middleware
//!
//! You can call the [`with`](EndpointExt::with) method on the [`Endpoint`] to
//! apply a middleware to an endpoint. It actually converts the original
//! endpoint to a new endpoint.
//! ```
//! use poem::{handler, middleware::Tracing, EndpointExt, Route};
//!
//! #[handler]
//! fn index() {}
//!
//! let app = Route::new().at("/", index).with(Tracing);
//! ```
//!
//! You can create your own middleware, see also [`Middleware`].
//!
//! # Crate features
//!
//! To avoid compiling unused dependencies, `Poem` gates certain features, all
//! of which are disabled by default:
//!
//! |Feature           |Description                     |
//! |------------------|--------------------------------|
//! | server | Server and listener APIs(enable by default) |
//! |compression  | Support decompress request body and compress response body |
//! |cookie            | Support for Cookie             |
//! |csrf | Support for Cross-Site Request Forgery (CSRF) protection |
//! |multipart         | Support for Multipart          |
//! |native-tls        | Support for HTTP server over TLS with [`native-tls`](https://crates.io/crates/native-tls)  |
//! |openssl-tls        | Support for HTTP server over TLS with [`openssl-tls`](https://crates.io/crates/openssl)  |
//! |opentelemetry     | Support for opentelemetry    |
//! |prometheus        | Support for Prometheus       |
//! |redis-session     | Support for RedisSession     |
//! |rustls            | Support for HTTP server over TLS with [`rustls`](https://crates.io/crates/rustls)  |
//! |session           | Support for session    |
//! |sse               | Support Server-Sent Events (SSE)       |
//! |tempfile          | Support for [`tempfile`](https://crates.io/crates/tempfile) |
//! |tower-compat      | Adapters for `tower::Layer` and `tower::Service`. |
//! |websocket         | Support for WebSocket          |
//! | anyhow        | Integrate with the [`anyhow`](https://crates.io/crates/anyhow) crate. |
//! | eyre06        | Integrate with version 0.6.x of the [`eyre`](https://crates.io/crates/eyre) crate. |
//! | i18n          | Support for internationalization |
//! | acme | Support for ACME(Automatic Certificate Management Environment) |
//! | tokio-metrics | Integrate with the [`tokio-metrics`](https://crates.io/crates/tokio-metrics) crate. |
//! | embed  | Integrate with [`rust-embed`](https://crates.io/crates/rust-embed) crate. |
//! | xml | Integrate with [`quick-xml`](https://crates.io/crates/quick-xml) crate. |

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod endpoint;
pub mod error;
#[cfg(feature = "i18n")]
#[cfg_attr(docsrs, doc(cfg(feature = "i18n")))]
pub mod i18n;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub mod listener;
pub mod middleware;
#[cfg(feature = "session")]
#[cfg_attr(docsrs, doc(cfg(feature = "session")))]
pub mod session;
#[cfg(feature = "test")]
#[cfg_attr(docsrs, doc(cfg(feature = "test")))]
pub mod test;
pub mod web;

#[doc(inline)]
pub use http;

mod addr;
mod body;
mod request;
mod response;
mod route;
#[cfg(feature = "server")]
mod server;

pub use addr::Addr;
pub use async_trait::async_trait;
pub use body::Body;
pub use endpoint::{Endpoint, EndpointExt, IntoEndpoint};
pub use error::{Error, Result};
pub use middleware::Middleware;
pub use poem_derive::handler;
pub use request::{OnUpgrade, Request, RequestBuilder, RequestParts, Upgraded};
pub use response::{Response, ResponseBuilder, ResponseParts};
pub use route::{
    connect, delete, get, head, options, patch, post, put, trace, Route, RouteDomain, RouteMethod,
    RouteScheme,
};
#[cfg(feature = "server")]
pub use server::Server;
pub use web::{FromRequest, IntoResponse, RequestBody};
