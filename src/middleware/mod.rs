//! Commonly used middleware.

mod add_data;
mod strip_prefix;

pub use add_data::AddData;
pub use strip_prefix::StripPrefix;

use crate::Endpoint;

/// Represents a middleware trait.
pub trait Middleware<E> {
    /// New endpoint type.
    ///
    /// If you don't know what type to use, then you can use [`Box<dyn Endpoint>`], which will bring
    /// some performance loss, but it is insignificant.
    type Output: Endpoint;

    /// Transform the input [EndPoint] to another one.
    fn transform(&self, ep: E) -> Self::Output;
}
