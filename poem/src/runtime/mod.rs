//! Runtime types

use std::future::Future;

#[cfg(target_os = "wasi")]
#[cfg_attr(docsrs, doc(cfg(target_family = "wasi")))]
pub mod wasi;

pub use tokio;

#[cfg(not(target_os = "wasi"))]
pub fn spawn<T>(future: T)
where
    T::Output: Send + 'static,
    T: Future + Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(target_os = "wasi")]
pub fn spawn<T: Future + Send + 'static>(future: T)
where
    T: Future + Send + 'static,
{
    wasi::spawn(future).detach();
}
