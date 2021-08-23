//! Commonly used middleware.

mod add_data;
mod cors;
mod set_header;
#[cfg(feature = "tracing")]
mod tracing;

pub use add_data::AddData;
pub use cors::Cors;
pub use set_header::SetHeader;

#[cfg(feature = "tracing")]
pub use self::tracing::Tracing;
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
