use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use priority_queue::PriorityQueue;
use serde_json::Value;

use crate::{session::SessionStorage, Result};

struct InnerStorage {
    sessions: HashMap<String, BTreeMap<String, Value>>,
    timeout_queue: PriorityQueue<String, Reverse<Instant>>,
}

impl InnerStorage {
    fn cleanup(&mut self) {
        loop {
            let now = Instant::now();
            if let Some((_, expire_at)) = self.timeout_queue.peek() {
                if expire_at.0 > now {
                    break;
                }
                if let Some((session_id, _)) = self.timeout_queue.pop() {
                    self.sessions.remove(&session_id);
                }
            } else {
                break;
            }
        }
    }
}

/// A session storage using memory.
pub struct MemoryStorage {
    inner: Arc<Mutex<InnerStorage>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let inner = Arc::new(Mutex::new(InnerStorage {
            sessions: HashMap::new(),
            timeout_queue: PriorityQueue::new(),
        }));
        tokio::spawn({
            let inner = Arc::downgrade(&inner);
            async move {
                loop {
                    match inner.upgrade() {
                        Some(inner) => inner.lock().cleanup(),
                        None => return,
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });
        Self { inner }
    }
}

impl MemoryStorage {
    /// Create a `MemoryStorage`.
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait::async_trait]
impl SessionStorage for MemoryStorage {
    async fn load_session(&self, session_id: &str) -> Result<Option<BTreeMap<String, Value>>> {
        let inner = self.inner.lock();
        Ok(inner.sessions.get(session_id).cloned())
    }

    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> Result<()> {
        let mut inner = self.inner.lock();
        inner.timeout_queue.remove(session_id);
        inner
            .sessions
            .insert(session_id.to_string(), entries.clone());
        if let Some(expires) = expires {
            inner
                .timeout_queue
                .push(session_id.to_string(), Reverse(Instant::now() + expires));
        }
        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut inner = self.inner.lock();
        inner.sessions.remove(session_id);
        inner.timeout_queue.remove(session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::{
            test_harness::{index, TestClient},
            CookieConfig, ServerSession,
        },
        EndpointExt, Route,
    };

    #[tokio::test]
    async fn memory_session() {
        let app = Route::new().at("/:action", index).with(ServerSession::new(
            CookieConfig::default(),
            MemoryStorage::new(),
        ));
        let mut client = TestClient::default();

        client.call(&app, 0).await;
        client.assert_cookies(vec![]);

        client.call(&app, 1).await;
        client.call(&app, 2).await;
        client.call(&app, 7).await;
        client.call(&app, 6).await;
        client.call(&app, 3).await;
        client.call(&app, 4).await;
        client.call(&app, 5).await;
        client.assert_cookies(vec![]);
    }

    #[tokio::test]
    async fn timeout() {
        let storage = MemoryStorage::new();
        let mut values = BTreeMap::new();
        values.insert("value".to_string(), "1".into());

        storage
            .update_session("a", &values, Some(Duration::from_secs(2)))
            .await
            .unwrap();
        storage
            .update_session("b", &values, Some(Duration::from_secs(1)))
            .await
            .unwrap();
        storage
            .update_session("c", &values, Some(Duration::from_secs(3)))
            .await
            .unwrap();

        assert_eq!(
            storage.load_session("a").await.unwrap(),
            Some(values.clone())
        );
        assert_eq!(
            storage.load_session("b").await.unwrap(),
            Some(values.clone())
        );
        assert_eq!(
            storage.load_session("c").await.unwrap(),
            Some(values.clone())
        );

        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert_eq!(
            storage.load_session("a").await.unwrap(),
            Some(values.clone())
        );
        assert_eq!(storage.load_session("b").await.unwrap(), None);
        assert_eq!(
            storage.load_session("c").await.unwrap(),
            Some(values.clone())
        );

        tokio::time::sleep(Duration::from_millis(1000)).await;
        assert_eq!(storage.load_session("a").await.unwrap(), None);
        assert_eq!(storage.load_session("b").await.unwrap(), None);
        assert_eq!(
            storage.load_session("c").await.unwrap(),
            Some(values.clone())
        );

        tokio::time::sleep(Duration::from_millis(1000)).await;
        assert_eq!(storage.load_session("a").await.unwrap(), None);
        assert_eq!(storage.load_session("b").await.unwrap(), None);
        assert_eq!(storage.load_session("c").await.unwrap(), None);
    }
}
