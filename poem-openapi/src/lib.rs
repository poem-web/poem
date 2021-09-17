//! OpenAPI support for Poem.

#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod auth;
mod base;
mod error;
mod openapi;
#[doc(hidden)]
pub mod param;
pub mod payload;
#[doc(hidden)]
pub mod registry;
pub mod types;
#[doc(hidden)]
pub mod ui;
#[doc(hidden)]
pub mod validation;

pub use base::{ApiRequest, ApiResponse, CombinedAPI, OpenApi, SecurityScheme, Tags};
pub use error::ParseRequestError;
pub use openapi::OpenApiService;
#[doc(hidden)]
pub use poem;
#[doc = include_str!("docs/request.md")]
pub use poem_openapi_derive::ApiRequest;
#[doc = include_str!("docs/response.md")]
pub use poem_openapi_derive::ApiResponse;
#[doc = include_str!("docs/enum.md")]
pub use poem_openapi_derive::Enum;
#[doc = include_str!("docs/multipart.md")]
pub use poem_openapi_derive::Multipart;
#[doc = include_str!("docs/object.md")]
pub use poem_openapi_derive::Object;
#[doc = include_str!("docs/openapi.md")]
pub use poem_openapi_derive::OpenApi;
pub use poem_openapi_derive::SecurityScheme;
#[doc = include_str!("docs/tags.md")]
pub use poem_openapi_derive::Tags;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use serde_json;
