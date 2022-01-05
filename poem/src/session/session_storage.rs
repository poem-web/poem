use std::{collections::BTreeMap, time::Duration};

use serde_json::Value;

use crate::Result;

/// Represents a back-end session storage.
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    /// Load session entries.
    async fn load_session(&self, session_id: &str) -> Result<Option<BTreeMap<String, Value>>>;

    /// Insert or update a session.
    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> Result<()>;

    /// Remove a session by session id.
    async fn remove_session(&self, session_id: &str) -> Result<()>;
}
