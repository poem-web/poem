use std::{collections::BTreeMap, future::Future, time::Duration};

use serde_json::Value;

use crate::Result;

/// Represents a back-end session storage.
pub trait SessionStorage: Send + Sync {
    /// Load session entries.
    fn load_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> impl Future<Output = Result<Option<BTreeMap<String, Value>>>> + Send + 'a;

    /// Insert or update a session.
    fn update_session<'a>(
        &'a self,
        session_id: &'a str,
        entries: &'a BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> impl Future<Output = Result<()>> + Send + 'a;

    /// Remove a session by session id.
    fn remove_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> impl Future<Output = Result<()>> + Send + 'a;
}
