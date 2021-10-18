use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use parking_lot::Mutex;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};

use crate::{
    middleware::{CookieJarManager, CookieJarManagerEndpoint},
    session::{CookieConfig, Session, SessionStatus},
    Endpoint, Middleware, Request,
};

/// Use memory for session storage.
pub struct MemorySession {
    config: Arc<CookieConfig>,
    storage: Arc<Mutex<HashMap<String, BTreeMap<String, String>>>>,
}

impl MemorySession {
    /// Create a `MemorySession` middleware.
    pub fn new(config: CookieConfig) -> Self {
        Self {
            config: Arc::new(config),
            storage: Default::default(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for MemorySession {
    type Output = CookieJarManagerEndpoint<MemorySessionEndpoint<E>>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManager::new().transform(MemorySessionEndpoint {
            inner: ep,
            config: self.config.clone(),
            storage: self.storage.clone(),
        })
    }
}

/// Endpoint for `MemorySession` middleware.
pub struct MemorySessionEndpoint<E> {
    inner: E,
    config: Arc<CookieConfig>,
    storage: Arc<Mutex<HashMap<String, BTreeMap<String, String>>>>,
}

fn generate_session_id() -> String {
    let value = std::iter::repeat(())
        .map(|()| OsRng.sample(Alphanumeric))
        .take(32)
        .collect::<Vec<_>>();
    String::from_utf8(value).unwrap_or_default()
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for MemorySessionEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Self::Output {
        let cookie_jar = req.cookie().clone();
        let session_id = self.config.get_cookie_value(&cookie_jar);
        let session = match session_id
            .as_ref()
            .and_then(|id| self.storage.lock().get(id).cloned())
            .map(Session::new)
        {
            Some(session) => session,
            None => Session::default(),
        };

        req.extensions_mut().insert(session.clone());
        let resp = self.inner.call(req).await;

        match session.status() {
            SessionStatus::Changed => match session_id {
                Some(session_id) => {
                    self.storage.lock().insert(session_id, session.entries());
                }
                None => {
                    let session_id = generate_session_id();
                    self.config.set_cookie_value(&cookie_jar, &session_id);
                    self.storage.lock().insert(session_id, session.entries());
                }
            },
            SessionStatus::Renewed => {
                if let Some(session_id) = session_id {
                    self.storage.lock().remove(&session_id);
                }

                let session_id = generate_session_id();
                self.config.set_cookie_value(&cookie_jar, &session_id);
                self.storage.lock().insert(session_id, session.entries());
            }
            SessionStatus::Purged => {
                if let Some(session_id) = session_id {
                    self.storage.lock().remove(&session_id);
                }
                self.config.remove_cookie(&cookie_jar);
            }
            SessionStatus::Unchanged => {}
        };

        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::test_harness::{index, TestClient},
        EndpointExt, Route,
    };

    #[tokio::test]
    async fn cookie_session() {
        let app = Route::new()
            .at("/:action", index)
            .with(MemorySession::new(CookieConfig::default()));
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
}
