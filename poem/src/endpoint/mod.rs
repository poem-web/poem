//! Endpoint related types.

mod after;
mod and_then;
mod around;
mod before;
mod catch_all_error;
mod catch_error;
#[allow(clippy::module_inception)]
mod endpoint;
mod inspect_all_err;
mod inspect_err;
mod map;
mod map_to_response;
#[cfg(feature = "prometheus")]
mod prometheus_exporter;
#[cfg(feature = "static-files")]
mod static_files;
#[cfg(feature = "tower-compat")]
mod tower_compat;

pub use after::After;
pub use and_then::AndThen;
pub use around::Around;
pub use before::Before;
pub use catch_all_error::CatchAllError;
pub use catch_error::CatchError;
pub use endpoint::{make, make_sync, BoxEndpoint, Endpoint, EndpointExt, IntoEndpoint};
pub use inspect_all_err::InspectAllError;
pub use inspect_err::InspectError;
pub use map::Map;
pub use map_to_response::MapToResponse;
#[cfg(feature = "prometheus")]
pub use prometheus_exporter::PrometheusExporter;
#[cfg(feature = "static-files")]
pub use static_files::{StaticFileEndpoint, StaticFilesEndpoint};
#[cfg(feature = "tower-compat")]
pub use tower_compat::TowerCompatExt;
