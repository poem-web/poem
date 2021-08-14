//! Poem is a full-featured and easy-to-use web framework with the Rust programming language.
//!
//! # Usage
//!
//! Depend on poem in Cargo.toml:
//!
//! ```toml
//! poem = "0.1"
//! ```
//!
//! # Example
//!
//! ```no_run
//! use poem::web::Path;
//! use poem::prelude::*;
//!
//! async fn hello(Path(name): Path<String>) -> String {
//!     format!("hello: {}", name)
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = route().at("/hello/:name", get(hello));
//!     serve(app).run("127.0.0.1:3000").await.unwrap();
//! }
//! ```
//!
//! # Features
//!
//! To avoid compiling unused dependencies, Poem gates certain features, all of which are disabled by default:
//!
//! |Feature           |Description                     |
//! |------------------|--------------------------------|
//! |websocket         |Support for WebSocket           |
//! |multipart         |Support for Multipart           |
//! |tls               |Support HTTP server over TLS    |

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod error;
pub mod middleware;
pub mod route;
pub mod web;
#[cfg(feature = "websocket")]
#[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
pub mod websocket;

mod body;
mod endpoint;
mod request;
mod response;
mod route_recognizer;
mod server;

#[doc(inline)]
pub use http;

pub use server::Server;
#[cfg(feature = "tls")]
pub use server::TlsServer;

/// Re-exports of important traits, types, and functions used with Poem.
pub mod prelude {
    use super::*;

    pub use body::Body;
    pub use endpoint::{Endpoint, EndpointExt, FnHandler};
    pub use error::{Error, Result};
    pub use middleware::Middleware;
    pub use request::{Request, RequestBuilder};
    pub use response::{Response, ResponseBuilder};
    pub use route::{connect, delete, get, head, options, patch, post, put, route, trace};
    pub use server::serve;
    pub use web::{FromRequest, IntoResponse};
}
