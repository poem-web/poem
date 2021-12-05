//! Session storage using database for Poem
//!
//! # Crate features
//!
//! ## [`sqlx`](https://crates.io/crates/sqlx)
//!
//! | feature                   | database | tls        |
//! |---------------------------|----------|------------|
//! | sqlx-mysql-rustls         | mysql    | rustls     |
//! | sqlx-mysql-native-tls     | mysql    | native-tls |
//! | sqlx-postgres-rustls      | postgres | rustls     |
//! | sqlx-postgres-native-tls  | postgres | native-tls |
//! | sqlx-sqlite-rustls        | sqlite   | rustls     |
//! | sqlx-sqlite-native-tls    | sqlite   | native-tls |
//!
//! ## Example
//!
//! ```rust,ignore
//! use poem::session::{CookieConfig, ServerSession, Session};
//! use poem_dbsession::{sqlx::MysqlSessionStorage, DatabaseConfig};
//! use sqlx::MySqlPool;
//!
//! #[handler]
//! fn index(session: &Session) {
//!     todo!()
//! }
//!
//! let pool = MySqlPool::connect("mysql://root:123456@localhost/my_database")
//!     .await
//!     .unwrap();
//! let storage = MysqlSessionStorage::try_new(DatabaseConfig::new(), pool).await.unwrap();
//! let route = Route::new().at("/", index).with(ServerSession::new(CookieConfig::new(),storage));
//! ```

#![doc(html_favicon_url = "https://poem.rs/assets/favicon.ico")]
#![doc(html_logo_url = "https://poem.rs/assets/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

#[cfg(any(
    feature = "sqlx-mysql-rustls",
    feature = "sqlx-mysql-native-tls",
    feature = "sqlx-postgres-rustls",
    feature = "sqlx-postgres-native-tls",
    feature = "sqlx-sqlite-rustls",
    feature = "sqlx-sqlite-tls"
))]
pub mod sqlx;

mod config;
#[cfg(test)]
mod test_harness;

pub use config::DatabaseConfig;
