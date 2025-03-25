use std::{collections::BTreeMap, time::Duration};

use redis::{Cmd, aio::ConnectionLike};
use serde_json::Value;

use crate::{Result, error::RedisSessionError, session::session_storage::SessionStorage};

/// A session storage using redis.
///
/// # Errors
///
/// - [`RedisSessionError`]
#[cfg_attr(docsrs, doc(cfg(feature = "redis-session")))]
pub struct RedisStorage<T> {
    connection: T,
}

impl<T> RedisStorage<T> {
    /// Create a `RedisStorage`.
    pub fn new(connection: T) -> Self {
        Self { connection }
    }
}

impl<T: ConnectionLike + Clone + Sync + Send> SessionStorage for RedisStorage<T> {
    async fn load_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> Result<Option<BTreeMap<String, Value>>> {
        let data: Option<String> = Cmd::get(session_id)
            .query_async(&mut self.connection.clone())
            .await
            .map_err(RedisSessionError::Redis)?;

        match data {
            Some(data) => {
                #[cfg(not(feature = "sonic-rs"))]
                let map = serde_json::from_str::<BTreeMap<String, Value>>(&data);
                #[cfg(feature = "sonic-rs")]
                let map = sonic_rs::from_str::<BTreeMap<String, Value>>(&data);
                match map {
                    Ok(entries) => Ok(Some(entries)),
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn update_session<'a>(
        &'a self,
        session_id: &'a str,
        entries: &'a BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> Result<()> {
        #[cfg(not(feature = "sonic-rs"))]
        let value = serde_json::to_string(entries).unwrap_or_default();
        #[cfg(feature = "sonic-rs")]
        let value = sonic_rs::to_string(entries).unwrap_or_default();
        let cmd = match expires {
            Some(expires) => Cmd::set_ex(session_id, value, expires.as_secs()),
            None => Cmd::set(session_id, value),
        };
        cmd.query_async::<()>(&mut self.connection.clone())
            .await
            .map_err(RedisSessionError::Redis)?;
        Ok(())
    }

    async fn remove_session<'a>(&'a self, session_id: &'a str) -> Result<()> {
        Cmd::del(session_id)
            .query_async::<()>(&mut self.connection.clone())
            .await
            .map_err(RedisSessionError::Redis)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use redis::{Client, ConnectionLike, aio::ConnectionManager};

    use super::*;
    use crate::{
        EndpointExt, Route,
        session::{
            CookieConfig, ServerSession,
            test_harness::{TestClient, index},
        },
    };

    #[tokio::test]
    async fn redis_session() {
        let mut client = match Client::open("redis://127.0.0.1/") {
            Ok(client) => client,
            Err(_) => return,
        };
        if !client.check_connection() {
            panic!("redis server is not running");
        }

        let app = Route::new().at("/:action", index).with(ServerSession::new(
            CookieConfig::default(),
            RedisStorage::new(ConnectionManager::new(client).await.unwrap()),
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
