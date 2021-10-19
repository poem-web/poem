//! Session management.

mod cookie_config;
mod cookie_session;
#[cfg(feature = "redis-session")]
mod redis_storage;
mod server_session;
#[allow(clippy::module_inception)]
mod session;
mod session_storage;
#[cfg(test)]
pub(crate) mod test_harness;

pub use cookie_config::{CookieConfig, CookieSecurity};
pub use cookie_session::{CookieSession, CookieSessionEndpoint};
#[cfg(feature = "redis-session")]
pub use redis_storage::RedisStorage;
pub use server_session::{ServerSession, ServerSessionEndpoint};
pub use session::{Session, SessionStatus};
pub use session_storage::SessionStorage;
