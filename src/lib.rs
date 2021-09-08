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
//! use poem::{handler, route, web::Path, Server};
//!
//! #[handler]
//! fn hello(Path(name): Path<String>) -> String {
//!     format!("hello: {}", name)
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     use poem::route::get;
//!     let app = route().at("/hello/:name", get(hello));
//!     let server = Server::bind("127.0.0.1:3000").await.unwrap();
//!     server.run(app).await.unwrap();
//! }
//! ```
//!
//! # Features
//!
//! To avoid compiling unused dependencies, Poem gates certain features, all of
//! which are disabled by default:
//!
//! |Feature           |Description                     |
//! |------------------|--------------------------------|
//! |websocket         | Support for WebSocket          |
//! |multipart         | Support for Multipart          |
//! |sse               | Support Server-Sent Events (SSE)       |
//! |tls               | Support for HTTP server over TLS   |
//! |typed-headers     | Support for [`typed-headers`](https://crates.io/crates/typed-headers)    |
//! |tracing           | Support for Tracing middleware |
//! |tempfile          | Support for [`tempfile`](https://crates.io/crates/tempfile) |

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod endpoint;
pub mod error;
pub mod middleware;
pub mod route;
pub mod service;
pub mod web;

#[doc(inline)]
pub use http;

mod body;
mod request;
mod response;
mod route_recognizer;
mod server;

pub use async_trait::async_trait;
pub use body::Body;
pub use endpoint::{Endpoint, EndpointExt, IntoEndpoint};
pub use error::{Error, Result};
pub use middleware::Middleware;
pub use poem_derive::handler;
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder};
pub use route::{route, RouteMethod};
pub use server::Server;
#[cfg(feature = "tls")]
pub use server::TlsServer;
pub use web::{FromRequest, IntoResponse, RequestBody};
