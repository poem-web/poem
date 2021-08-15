//! Commonly used middleware.

mod add_data;
mod cors;
#[cfg(feature = "logger")]
#[cfg_attr(docsrs, doc(cfg(feature = "logger")))]
pub mod log;
mod strip_prefix;

pub use add_data::AddData;
pub use cors::Cors;
pub use strip_prefix::StripPrefix;

#[cfg(feature = "logger")]
pub use self::log::Logger;

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
    fn transform(self, ep: E) -> Self::Output;
}
