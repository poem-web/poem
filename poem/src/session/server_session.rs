use std::sync::Arc;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{thread_rng, Rng};

use crate::{
    middleware::{CookieJarManager, CookieJarManagerEndpoint},
    session::{session_storage::SessionStorage, CookieConfig, Session, SessionStatus},
    Endpoint, Middleware, Request, Result,
};

/// Middleware for server-side session.
pub struct ServerSession<T> {
    config: Arc<CookieConfig>,
    storage: Arc<T>,
}

impl<T> ServerSession<T> {
    /// Create a `ServerSession` middleware.
    pub fn new(config: CookieConfig, storage: T) -> Self {
        Self {
            config: Arc::new(config),
            storage: Arc::new(storage),
        }
    }
}

impl<T: SessionStorage, E: Endpoint> Middleware<E> for ServerSession<T> {
    type Output = CookieJarManagerEndpoint<ServerSessionEndpoint<T, E>>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManager::new().transform(ServerSessionEndpoint {
            inner: ep,
            config: self.config.clone(),
            storage: self.storage.clone(),
        })
    }
}

/// Session key generation routine that follows [OWASP recommendations].
///
/// [OWASP recommendations]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-entropy
fn generate_session_id() -> String {
    let random_bytes = thread_rng().gen::<[u8; 32]>();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Endpoint for `ServerSession` middleware.
pub struct ServerSessionEndpoint<T, E> {
    inner: E,
    config: Arc<CookieConfig>,
    storage: Arc<T>,
}

#[async_trait::async_trait]
impl<T, E> Endpoint for ServerSessionEndpoint<T, E>
where
    T: SessionStorage,
    E: Endpoint,
{
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let cookie_jar = req.cookie().clone();
        let mut session_id = self.config.get_cookie_value(&cookie_jar);
        let session = match &session_id {
            Some(id) => match self.storage.load_session(id).await? {
                Some(entries) => Session::new(entries),
                None => {
                    session_id = None;
                    Session::default()
                }
            },
            None => Session::default(),
        };

        req.extensions_mut().insert(session.clone());
        let resp = self.inner.call(req).await?;

        match session.status() {
            SessionStatus::Changed => match session_id {
                Some(session_id) => {
                    self.storage
                        .update_session(&session_id, &session.entries(), self.config.ttl())
                        .await?;
                }
                None => {
                    let session_id = generate_session_id();
                    self.config.set_cookie_value(&cookie_jar, &session_id);
                    self.storage
                        .update_session(&session_id, &session.entries(), self.config.ttl())
                        .await?;
                }
            },
            SessionStatus::Renewed => {
                if let Some(session_id) = session_id {
                    self.storage.remove_session(&session_id).await?;
                }

                let session_id = generate_session_id();
                self.config.set_cookie_value(&cookie_jar, &session_id);
                self.storage
                    .update_session(&session_id, &session.entries(), self.config.ttl())
                    .await?;
            }
            SessionStatus::Purged => {
                if let Some(session_id) = session_id {
                    self.storage.remove_session(&session_id).await?;
                    self.config.remove_cookie(&cookie_jar);
                }
            }
            SessionStatus::Unchanged => {}
        };

        Ok(resp)
    }
}
