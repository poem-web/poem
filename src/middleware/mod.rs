//! Commonly used middleware.
mod add_data;
#[cfg(feature = "logger")]
mod log;
mod strip_prefix;

#[cfg(feature = "logger")]
pub use log::Logger;

pub use add_data::AddData;
pub use strip_prefix::StripPrefix;

use crate::endpoint::Endpoint;

/// Represents a middleware trait.
pub trait Middleware<E: Endpoint> {
    /// New endpoint type.
    ///
    /// If you don't know what type to use, then you can use [`Box<dyn
    /// Endpoint>`], which will bring some performance loss, but it is
    /// insignificant.
    type Output: Endpoint;

    /// Transform the input [`Endpoint`] to another one.
    fn transform(&self, ep: E) -> Self::Output;
}
