//! GRPC server for Poem

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod client;
#[doc(hidden)]
pub mod server;
#[doc(hidden)]
pub mod service;

pub mod codec;
pub mod metadata;

mod encoding;
mod health;
mod reflection;
mod request;
mod response;
mod route;
mod status;
mod streaming;
#[cfg(test)]
mod test_harness;

pub use client::{ClientBuilderError, ClientConfig, ClientConfigBuilder};
pub use health::{health_service, HealthReporter, ServingStatus};
pub use metadata::Metadata;
pub use reflection::Reflection;
pub use request::Request;
pub use response::Response;
pub use route::RouteGrpc;
pub use service::Service;
pub use status::{Code, Status};
pub use streaming::Streaming;
