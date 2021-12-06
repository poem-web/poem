#![allow(dead_code)]

/// A configuration for database.
pub struct DatabaseConfig {
    pub(crate) table_name: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            table_name: "poem_sessions".to_string(),
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
        }
    }
}
