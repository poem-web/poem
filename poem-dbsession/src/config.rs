#![allow(dead_code)]

use std::time::Duration;

/// A configuration for database.
pub struct DatabaseConfig {
    pub(crate) table_name: String,
    pub(crate) cleanup_period: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            table_name: "poem_sessions".to_string(),
            cleanup_period: Duration::from_secs(60),
        }
    }
}

impl DatabaseConfig {
    /// Create an [`DatabaseConfig`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Specifies the table name.
    pub fn table_name(self, table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            ..self
        }
    }

    /// Specify the period to clean up expired sessions.
    pub fn cleanup_period(self, cleanup_period: Duration) -> Self {
        Self {
            cleanup_period,
            ..self
        }
    }
}
