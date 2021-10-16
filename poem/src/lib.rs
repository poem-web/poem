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
//! |cookie            | Support for Cookie             |
//! |websocket         | Support for WebSocket          |
//! |multipart         | Support for Multipart          |
//! |sse               | Support Server-Sent Events (SSE)       |
//! |tls               | Support for HTTP server over TLS   |
//! |tempfile          | Support for [`tempfile`](https://crates.io/crates/tempfile) |
//! |tower-compat      | Adapters for `tower::Layer` and `tower::Service`. |

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
pub mod service;
pub mod web;

#[doc(inline)]
pub use http;

mod body;
mod request;
mod response;
mod route;
mod server;

pub use async_trait::async_trait;
pub use body::Body;
pub use endpoint::{Endpoint, EndpointExt, IntoEndpoint};
pub use error::{Error, Result};
pub use middleware::Middleware;
pub use poem_derive::handler;
pub use request::{Request, RequestBuilder, RequestParts};
pub use response::{Response, ResponseBuilder, ResponseParts};
pub use route::{
    connect, delete, get, head, options, patch, post, put, trace, Route, RouteDomain, RouteMethod,
};
pub use server::Server;
pub use web::{FromRequest, IntoResponse, RequestBody};
