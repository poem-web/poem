use std::{collections::BTreeMap, sync::Arc};

use serde_json::Value;

use crate::{
    middleware::{CookieJarManager, CookieJarManagerEndpoint},
    session::{CookieConfig, Session, SessionStatus},
    Endpoint, Middleware, Request, Result,
};

/// Middleware for client-side(cookie) session.
pub struct CookieSession {
    config: Arc<CookieConfig>,
}

impl CookieSession {
    /// Create a `CookieSession` middleware.
    ///
    /// It stores the session data in a single cookie, and the serialized
    /// session data cannot exceed 4k bytes.
    pub fn new(config: CookieConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<E: Endpoint> Middleware<E> for CookieSession {
    type Output = CookieJarManagerEndpoint<CookieSessionEndpoint<E>>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManager::new().transform(CookieSessionEndpoint {
            inner: ep,
            config: self.config.clone(),
        })
    }
}

/// Endpoint for `CookieSession` middleware.
pub struct CookieSessionEndpoint<E> {
    inner: E,
    config: Arc<CookieConfig>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CookieSessionEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let cookie_jar = req.cookie().clone();
        let session = self
            .config
            .get_cookie_value(&cookie_jar)
            .and_then(|value| serde_json::from_str::<BTreeMap<String, Value>>(&value).ok())
            .map(Session::new)
            .unwrap_or_else(Session::default);

        req.extensions_mut().insert(session.clone());
        let resp = self.inner.call(req).await?;

        match session.status() {
            SessionStatus::Changed | SessionStatus::Renewed => {
                self.config.set_cookie_value(
                    &cookie_jar,
                    &serde_json::to_string(&session.entries()).unwrap_or_default(),
                );
            }
            SessionStatus::Purged => {
                self.config.remove_cookie(&cookie_jar);
            }
            SessionStatus::Unchanged => {}
        };

        Ok(resp)
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
            .with(CookieSession::new(CookieConfig::default()));
        let mut client = TestClient::default();

        client.call(&app, 0).await;
        client.assert_cookies(vec![]);

        client.call(&app, 1).await;
        client.assert_cookies(vec![("poem-session", r#"{"a":10,"b":20}"#)]);

        client.call(&app, 2).await;
        client.assert_cookies(vec![("poem-session", r#"{"a":10,"b":20,"c":30}"#)]);

        client.call(&app, 7).await;

        client.call(&app, 6).await;
        client.assert_cookies(vec![("poem-session", r#"{"a":10,"b":20,"c":30}"#)]);

        client.call(&app, 3).await;
        client.assert_cookies(vec![("poem-session", r#"{"a":10,"c":30}"#)]);

        client.call(&app, 4).await;
        client.assert_cookies(vec![("poem-session", r#"{}"#)]);

        client.call(&app, 5).await;
        client.assert_cookies(vec![]);
    }
}
