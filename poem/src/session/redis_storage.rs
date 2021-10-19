use std::{collections::BTreeMap, time::Duration};

use redis::{aio::ConnectionLike, AsyncCommands, Cmd};

use crate::{session::session_storage::SessionStorage, Error, Result};

/// A session storage using redis.
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
    async fn load_session(&self, session_id: &str) -> Result<BTreeMap<String, String>> {
        let data: String = self
            .connection
            .clone()
            .get(session_id)
            .await
            .map_err(Error::internal_server_error)?;
        Ok(serde_json::from_str::<BTreeMap<String, String>>(&data).unwrap_or_default())
    }

    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, String>,
        expires: Option<Duration>,
    ) -> Result<()> {
        let value = serde_json::to_string(entries).unwrap_or_default();
        let cmd = match expires {
            Some(expires) => Cmd::set_ex(&session_id, value, expires.as_secs() as usize),
            None => Cmd::set(&session_id, value),
        };
        cmd.query_async(&mut self.connection.clone())
            .await
            .map_err(Error::internal_server_error)?;
        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<()> {
        Cmd::del(session_id)
            .query_async(&mut self.connection.clone())
            .await
            .map_err(Error::internal_server_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use redis::{aio::ConnectionManager, Client};

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
        let client = Client::open("redis://127.0.0.1/").unwrap();
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
