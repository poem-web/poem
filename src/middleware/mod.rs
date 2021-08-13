//! Commonly used middleware.

mod add_data;
mod strip_prefix;

pub use add_data::AddData;
pub use strip_prefix::StripPrefix;

use crate::Endpoint;

/// Represents a middleware trait.
pub trait Middleware {
    /// Transform the input [EndPoint] to another one.
    fn transform<T: Endpoint>(&self, ep: T) -> Box<dyn Endpoint>;
}
