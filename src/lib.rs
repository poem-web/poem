#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub use http::Extensions;

pub use server::Server;

pub mod error;
pub mod middleware;
pub mod route;
pub mod uri;
pub mod web;
#[cfg(feature = "websocket")]
#[cfg_attr(docsrs, doc(cfg(feature = "websocket")))]
pub mod websocket;

mod body;
mod endpoint;
mod header;
mod method;
mod request;
mod response;
mod route_recognizer;
mod server;
mod status_code;
mod version;

pub use body::Body;
pub use endpoint::{Endpoint, EndpointExt, FnHandler};
pub use error::{Error, Result};
pub use header::{map::HeaderMap, name::HeaderName, value::HeaderValue};
pub use method::Method;
pub use middleware::Middleware;
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder};
pub use route::{connect, delete, get, head, options, patch, post, put, route, trace};
pub use status_code::StatusCode;
pub use version::Version;
pub use web::{FromRequest, IntoResponse};
