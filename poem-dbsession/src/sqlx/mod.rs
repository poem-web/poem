//! sqlx-backed session storages.

#[cfg(any(feature = "sqlx-mysql-rustls", feature = "sqlx-mysql-native-tls"))]
mod mysql;
#[cfg(any(feature = "sqlx-postgres-rustls", feature = "sqlx-postgres-native-tls"))]
mod postgres;
#[cfg(any(feature = "sqlx-sqlite-rustls", feature = "sqlx-sqlite-native-tls"))]
mod sqlite;

#[cfg(any(feature = "sqlx-mysql-rustls", feature = "sqlx-mysql-native-tls"))]
pub use mysql::MysqlSessionStorage;
#[cfg(any(feature = "sqlx-postgres-rustls", feature = "sqlx-postgres-native-tls"))]
pub use postgres::PgSessionStorage;
#[cfg(any(feature = "sqlx-sqlite-rustls", feature = "sqlx-sqlite-native-tls"))]
pub use sqlite::SqliteSessionStorage;
