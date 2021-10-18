//! Session management.

mod cookie_config;
mod cookie_session;
#[cfg(feature = "redis-session")]
mod redis_session;
#[allow(clippy::module_inception)]
mod session;
#[cfg(test)]
pub(crate) mod test_harness;

pub use cookie_config::{CookieConfig, CookieSecurity};
pub use cookie_session::{CookieSession, CookieSessionEndpoint};
#[cfg(feature = "redis-session")]
pub use redis_session::{RedisSession, RedisSessionEndpoint};
pub use session::{Session, SessionStatus};
