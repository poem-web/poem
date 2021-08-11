#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod middlewares;
pub mod route;
pub mod uri;
pub mod web;

mod body;
mod endpoint;
mod header;
mod method;
mod middleware;
mod request;
mod response;
mod route_recognizer;
mod server;
mod status_code;
mod version;

pub use http::Extensions;

pub use body::Body;
pub use endpoint::{Endpoint, EndpointExt, FnHandler};
pub use error::{Error, Result};
pub use header::{map::HeaderMap, name::HeaderName, value::HeaderValue};
pub use method::Method;
pub use middleware::Middleware;
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder};
pub use server::Server;
pub use status_code::StatusCode;
pub use version::Version;
pub use web::{FromRequest, IntoResponse};
