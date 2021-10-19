use std::{collections::BTreeMap, time::Duration};

use crate::Result;

/// Represents a back-end session storage.
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    /// Load session entries.
    async fn load_session(&self, session_id: &str) -> Result<BTreeMap<String, String>>;

    /// Insert or update a session.
    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, String>,
        expires: Option<Duration>,
    ) -> Result<()>;

    /// Remove a session by session id.
    async fn remove_session(&self, session_id: &str) -> Result<()>;
}
