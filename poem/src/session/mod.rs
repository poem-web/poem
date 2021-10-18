//! Session management.

mod cookie_config;
mod cookie_session;
mod memory_session;
#[allow(clippy::module_inception)]
mod session;
#[cfg(test)]
pub(crate) mod test_harness;

pub use cookie_config::{CookieConfig, CookieSecurity};
pub use cookie_session::{CookieSession, CookieSessionEndpoint};
pub use memory_session::{MemorySession, MemorySessionEndpoint};
pub use session::{Session, SessionStatus};
