//! Some commonly used services that implement
//! [`Endpoint`](crate::endpoint::Endpoint).

mod files;
#[cfg(feature = "tower-compat")]
mod tower_compat;

pub use files::Files;
#[cfg(feature = "tower-compat")]
pub use tower_compat::{TowerCompatEndpoint, TowerCompatExt};
