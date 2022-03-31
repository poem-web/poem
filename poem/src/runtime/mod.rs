//! Runtime types

#[cfg(target_os = "wasi")]
#[cfg_attr(docsrs, doc(cfg(target_family = "wasi")))]
pub mod wasi;

pub use tokio;
