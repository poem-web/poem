use std::{collections::BTreeMap, time::Duration};

use chrono::Utc;
use poem::{error::InternalServerError, session::SessionStorage, Result};
use serde_json::Value;
use sqlx::{sqlite::SqliteStatement, types::Json, Executor, SqlitePool, Statement};

use crate::DatabaseConfig;

const LOAD_SESSION_SQL: &str = r#"
    select session from {table_name}
        where id = ? and (expires is null or expires > ?)
    "#;

const UPDATE_SESSION_SQL: &str = r#"
    insert into {table_name} (id, session, expires) values (?, ?, ?)
        on conflict(id) do update set
            expires = excluded.expires,
            session = excluded.session
"#;

const REMOVE_SESSION_SQL: &str = r#"
    delete from {table_name} where id = ?
"#;

const CLEANUP_SQL: &str = r#"
    delete from {table_name} where expires < $1
"#;

/// Session storage using Sqlite.
///
/// # Errors
///
/// - [`sqlx::Error`]
///
/// # Create the table for session storage
///
/// ```sql
/// create table poem_sessions (
///     id text primary key not null,
///     expires integer null,
///     session text not null
/// )
/// ```
#[derive(Clone)]
pub struct SqliteSessionStorage {
    pool: SqlitePool,
    load_stmt: SqliteStatement<'static>,
    update_stmt: SqliteStatement<'static>,
    remove_stmt: SqliteStatement<'static>,
    cleanup_stmt: SqliteStatement<'static>,
}

impl SqliteSessionStorage {
    /// Create an [`SqliteSessionStorage`].
    pub async fn try_new(config: DatabaseConfig, pool: SqlitePool) -> sqlx::Result<Self> {
        let mut conn = pool.acquire().await?;

        let load_stmt = Statement::to_owned(
            &conn
                .prepare(&LOAD_SESSION_SQL.replace("{table_name}", &config.table_name))
                .await?,
        );

        let update_stmt = Statement::to_owned(
            &conn
                .prepare(&UPDATE_SESSION_SQL.replace("{table_name}", &config.table_name))
                .await?,
        );

        let remove_stmt = Statement::to_owned(
            &conn
                .prepare(&REMOVE_SESSION_SQL.replace("{table_name}", &config.table_name))
                .await?,
        );

        let cleanup_stmt = Statement::to_owned(
            &conn
                .prepare(&CLEANUP_SQL.replace("{table_name}", &config.table_name))
                .await?,
        );

        Ok(Self {
            pool,
            load_stmt,
            update_stmt,
            remove_stmt,
            cleanup_stmt,
        })
    }

    /// Cleanup expired sessions.
    pub async fn cleanup(&self) -> sqlx::Result<()> {
        let mut conn = self.pool.acquire().await?;
        self.cleanup_stmt
            .query()
            .bind(Utc::now())
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

#[poem::async_trait]
impl SessionStorage for SqliteSessionStorage {
    async fn load_session(&self, session_id: &str) -> Result<Option<BTreeMap<String, Value>>> {
        let mut conn = self.pool.acquire().await.map_err(InternalServerError)?;
        let res: Option<(Json<BTreeMap<String, Value>>,)> = self
            .load_stmt
            .query_as()
            .bind(session_id)
            .bind(Utc::now())
            .fetch_optional(&mut conn)
            .await
            .map_err(InternalServerError)?;
        Ok(res.map(|(value,)| value.0))
    }

    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, Value>,
        expires: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await.map_err(InternalServerError)?;

        let expires = match expires {
            Some(expires) => {
                Some(chrono::Duration::from_std(expires).map_err(InternalServerError)?)
            }
            None => None,
        };

        self.update_stmt
            .query()
            .bind(session_id)
            .bind(Json(entries))
            .bind(expires.map(|expires| Utc::now() + expires))
            .execute(&mut conn)
            .await
            .map_err(InternalServerError)?;
        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut conn = self.pool.acquire().await.map_err(InternalServerError)?;
        self.remove_stmt
            .query()
            .bind(session_id)
            .execute(&mut conn)
            .await
            .map_err(InternalServerError)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness;

    #[tokio::test]
    async fn test() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        sqlx::query(
            r#"
        create table poem_sessions (
            id text primary key not null,
            expires integer null,
            session text not null
        )
        "#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        let storage = SqliteSessionStorage::try_new(DatabaseConfig::new(), pool)
            .await
            .unwrap();

        let join_handle = tokio::spawn({
            let storage = storage.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    storage.cleanup().await.unwrap();
                }
            }
        });
        test_harness::test_storage(storage).await;
        join_handle.abort();
    }
}
