#[allow(clippy::module_inception)]
mod cache;
mod cache_storage;
mod memory_storage;

pub use cache::{Cache, CacheGetOptions, CacheSetOptions};
pub use cache_storage::CacheStorage;
pub use memory_storage::MemoryStorage;
