//! Poem is a full-featured and easy-to-use web framework with the Rust
//! programming language.
//!
//! # Usage
//!
//! Depend on poem in Cargo.toml:
//!
//! ```toml
//! poem = "*"
//! ```
//!
//! # Example
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
//! # Crate features
//!
//! To avoid compiling unused dependencies, Poem gates certain features, all of
//! which are disabled by default:
//!
//! |Feature           |Description                     |
//! |------------------|--------------------------------|
//! |compression  | Support decompress request body and compress response body |
//! |cookie            | Support for Cookie             |
//! |csrf | Support for Cross-Site Request Forgery (CSRF) protection |
//! |multipart         | Support for Multipart          |
//! |native-tls        | Support for HTTP server over TLS with [`native-tls`](https://crates.io/crates/native-tls)  |
//! |opentelemetry     | Support for opentelemetry    |
//! |prometheus        | Support for Prometheus       |
//! |redis-session     | Support for RedisSession     |
//! |rustls            | Support for HTTP server over TLS with [`rustls`](https://crates.io/crates/rustls)  |
//! |session           | Support for session    |
//! |sse               | Support Server-Sent Events (SSE)       |
//! |tempfile          | Support for [`tempfile`](https://crates.io/crates/tempfile) |
//! |tower-compat      | Adapters for `tower::Layer` and `tower::Service`. |
//! |websocket         | Support for WebSocket          |

#![doc(html_favicon_url = "https://poem.rs/assets/favicon.ico")]
#![doc(html_logo_url = "https://poem.rs/en/assets/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod endpoint;
pub mod error;
pub mod listener;
pub mod middleware;
#[cfg(feature = "session")]
#[cfg_attr(docsrs, doc(cfg(feature = "session")))]
pub mod session;
pub mod web;

#[doc(inline)]
pub use http;

mod addr;
mod body;
mod request;
mod response;
mod route;
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
};
pub use server::Server;
pub use web::{FromRequest, IntoResponse, RequestBody};
