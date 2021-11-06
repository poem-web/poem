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
//!     let listener = TcpListener::bind("127.0.0.1:3000");
//!     let server = Server::new(listener).await?;
//!     server.run(app).await
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
//! |compression | Support decompress request body and compress response body |
//! |cookie            | Support for Cookie             |
//! |multipart         | Support for Multipart          |
//! |opentelemetry     | Support for opentelemetry    |
//! |prometheus        | Support for Prometheus       |
//! |redis-session     | Support for RedisSession     |
//! |session           | Support for CookieSession    |
//! |sse               | Support Server-Sent Events (SSE)       |
//! |staticfiles       | Support for serve static files       |
//! |tempfile          | Support for [`tempfile`](https://crates.io/crates/tempfile) |
//! |template          | Support for [`askama`](https://crates.io/crates/askama)       |
//! |tls               | Support for HTTP server over TLS   |
//! |tower-compat      | Adapters for `tower::Layer` and `tower::Service`. |
//! |websocket         | Support for WebSocket          |

#![doc(html_favicon_url = "https://poem.rs/assets/favicon.ico")]
#![doc(html_logo_url = "https://poem.rs/assets/logo.png")]
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
