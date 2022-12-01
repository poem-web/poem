use std::{collections::BTreeMap, time::Duration};

use redis::{aio::ConnectionLike, AsyncCommands, Cmd};
use serde_json::Value;

use crate::{error::InternalServerError, session::session_storage::SessionStorage, Result};

/// A session storage using redis.
///
/// # Errors
///
/// - [`redis::RedisError`]
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

#[async_trait::async_trait]
impl<T: ConnectionLike + Clone + Sync + Send> SessionStorage for RedisStorage<T> {
    async fn load_session(&self, session_id: &str) -> Result<Option<BTreeMap<String, Value>>> {
        let data: Option<String> = self
            .connection
            .clone()
            .get(session_id)
            .await
            .map_err(InternalServerError)?;
        match data {
            Some(data) => match serde_json::from_str::<BTreeMap<String, Value>>(&data) {
                Ok(entries) => Ok(Some(entries)),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> Result<()> {
        let value = serde_json::to_string(entries).unwrap_or_default();
        let cmd = match expires {
            Some(expires) => Cmd::set_ex(session_id, value, expires.as_secs() as usize),
            None => Cmd::set(session_id, value),
        };
        cmd.query_async(&mut self.connection.clone())
            .await
            .map_err(InternalServerError)?;
        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<()> {
        Cmd::del(session_id)
            .query_async(&mut self.connection.clone())
            .await
            .map_err(InternalServerError)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use redis::{aio::ConnectionManager, Client, ConnectionLike};

    use super::*;
    use crate::{
        session::{
            test_harness::{index, TestClient},
            CookieConfig, ServerSession,
        },
        EndpointExt, Route,
    };

    #[tokio::test]
    async fn redis_session() {
        let mut client = match Client::open("redis://127.0.0.1/") {
            Ok(client) => client,
            Err(_) => return,
        };
        if !client.check_connection() {
            return;
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
