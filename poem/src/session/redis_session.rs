use std::{collections::BTreeMap, sync::Arc};

use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};
use redis::{aio::ConnectionLike, AsyncCommands, Cmd};

use crate::{
    middleware::{CookieJarManager, CookieJarManagerEndpoint},
    session::{CookieConfig, Session, SessionStatus},
    Endpoint, Error, Middleware, Request, Result,
};

/// Use redis for session storage.
#[cfg_attr(docsrs, doc(cfg(feature = "redis-session")))]
pub struct RedisSession<T> {
    config: Arc<CookieConfig>,
    connection: T,
}

impl<T: ConnectionLike + Clone + Sync + Send> RedisSession<T> {
    /// Create a `RedisSession` middleware.
    pub fn new(config: CookieConfig, connection: T) -> Self {
        Self {
            config: Arc::new(config),
            connection,
        }
    }
}

impl<T: ConnectionLike + Clone + Sync + Send, E: Endpoint> Middleware<E> for RedisSession<T> {
    type Output = CookieJarManagerEndpoint<RedisSessionEndpoint<T, E>>;

    fn transform(&self, ep: E) -> Self::Output {
        CookieJarManager::new().transform(RedisSessionEndpoint {
            inner: ep,
            config: self.config.clone(),
            connection: self.connection.clone(),
        })
    }
}

fn generate_session_id() -> String {
    let value = std::iter::repeat(())
        .map(|()| OsRng.sample(Alphanumeric))
        .take(32)
        .collect::<Vec<_>>();
    String::from_utf8(value).unwrap_or_default()
}

/// Endpoint for `RedisSession` middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "redis-session")))]
pub struct RedisSessionEndpoint<T, E> {
    inner: E,
    config: Arc<CookieConfig>,
    connection: T,
}

async fn set_session_to_redis<T: ConnectionLike>(
    cfg: &CookieConfig,
    connection: &mut T,
    session_id: &str,
    session: &Session,
) -> Result<()> {
    let value = serde_json::to_string(&session.entries()).unwrap_or_default();
    let cmd = match cfg.ttl() {
        Some(d) => Cmd::set_ex(&session_id, value, d.as_secs() as usize),
        None => Cmd::set(&session_id, value),
    };
    cmd.query_async(connection)
        .await
        .map_err(Error::internal_server_error)?;
    Ok(())
}

async fn remove_session_from_redis<T: ConnectionLike>(
    connection: &mut T,
    session_id: &str,
) -> Result<()> {
    Cmd::del(session_id)
        .query_async(connection)
        .await
        .map_err(Error::internal_server_error)?;
    Ok(())
}

#[async_trait::async_trait]
impl<T: ConnectionLike + Clone + Sync + Send, E: Endpoint> Endpoint for RedisSessionEndpoint<T, E> {
    type Output = Result<E::Output>;

    async fn call(&self, mut req: Request) -> Self::Output {
        let mut connection = self.connection.clone();
        let cookie_jar = req.cookie().clone();
        let session_id = self.config.get_cookie_value(&cookie_jar);
        let session = match &session_id {
            Some(session_id) => {
                let data: String = connection
                    .get(session_id)
                    .await
                    .map_err(Error::internal_server_error)?;
                let entries =
                    serde_json::from_str::<BTreeMap<String, String>>(&data).unwrap_or_default();
                Session::new(entries)
            }
            None => Session::default(),
        };

        req.extensions_mut().insert(session.clone());
        let resp = self.inner.call(req).await;

        match session.status() {
            SessionStatus::Changed => match session_id {
                Some(session_id) => {
                    set_session_to_redis(&self.config, &mut connection, &session_id, &session)
                        .await?;
                }
                None => {
                    let session_id = generate_session_id();
                    self.config.set_cookie_value(&cookie_jar, &session_id);
                    set_session_to_redis(&self.config, &mut connection, &session_id, &session)
                        .await?;
                }
            },
            SessionStatus::Renewed => {
                if let Some(session_id) = session_id {
                    remove_session_from_redis(&mut connection, &session_id).await?;
                }

                let session_id = generate_session_id();
                self.config.set_cookie_value(&cookie_jar, &session_id);
                set_session_to_redis(&self.config, &mut connection, &session_id, &session).await?;
            }
            SessionStatus::Purged => {
                if let Some(session_id) = session_id {
                    remove_session_from_redis(&mut connection, &session_id).await?;
                    self.config.remove_cookie(&cookie_jar);
                }
            }
            SessionStatus::Unchanged => {}
        };

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use redis::{aio::ConnectionManager, Client};

    use super::*;
    use crate::{
        session::test_harness::{index, TestClient},
        EndpointExt, Route,
    };

    #[tokio::test]
    async fn redis_session() {
        let client = Client::open("redis://127.0.0.1/").unwrap();
        let app = Route::new().at("/:action", index).with(RedisSession::new(
            CookieConfig::default(),
            ConnectionManager::new(client).await.unwrap(),
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
}
