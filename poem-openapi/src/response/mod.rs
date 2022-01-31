//! Commonly used response types.

#[cfg(feature = "static-files")]
mod static_file;

#[cfg(feature = "static-files")]
pub use static_file::StaticFileResponse;
