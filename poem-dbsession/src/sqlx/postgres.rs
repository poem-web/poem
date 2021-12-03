use std::{collections::BTreeMap, time::Duration};

use chrono::Utc;
use poem::{session::SessionStorage, Result};
use sqlx::{postgres::PgStatement, types::Json, Executor, PgPool, Statement};

use crate::{cleanup_task::CleanupTask, DatabaseConfig};

const LOAD_SESSION_SQL: &str = r#"
    select session from {table_name}
        where id = $1 and (expires is null or expires > $2)
    "#;

const UPDATE_SESSION_SQL: &str = r#"
    insert into {table_name} (id, session, expires) values ($1, $2, $3)
        on conflict(id) do update set
            expires = excluded.expires,
            session = excluded.session
"#;

const REMOVE_SESSION_SQL: &str = r#"
    delete from {table_name} where id = $1
"#;

const CLEANUP_SQL: &str = r#"
    delete from {table_name} where expires < $1
"#;

/// Session storage using Postgres.
///
/// # Create the table for session storage
///
/// ```sql
/// create table if not exists poem_sessions (
///     id varchar not null primary key,
///     expires timestamp with time zone null,
///     session jsonb not null
/// );
///
/// create index if not exists poem_sessions_expires_idx on poem_sessions (expires);
/// ```
pub struct PgSessionStorage {
    pool: PgPool,
    _cleanup_task: CleanupTask,
    load_stmt: PgStatement<'static>,
    update_stmt: PgStatement<'static>,
    remove_stmt: PgStatement<'static>,
}

impl PgSessionStorage {
    /// Create an [`PgSessionStorage`].
    pub async fn try_new(config: DatabaseConfig, pool: PgPool) -> Result<Self> {
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

        let cleanup_task = CleanupTask::new(config.cleanup_period, {
            let pool = pool.clone();
            move || {
                let pool = pool.clone();
                let stmt = cleanup_stmt.clone();
                async move {
                    if let Err(err) = do_cleanup(&pool, &stmt).await {
                        tracing::error!(error = %err, "failed to cleanup session storage");
                    }
                }
            }
        });

        Ok(Self {
            pool,
            _cleanup_task: cleanup_task,
            load_stmt,
            update_stmt,
            remove_stmt,
        })
    }
}

#[poem::async_trait]
impl SessionStorage for PgSessionStorage {
    async fn load_session(&self, session_id: &str) -> Result<Option<BTreeMap<String, String>>> {
        let mut conn = self.pool.acquire().await?;
        let res: Option<(Json<BTreeMap<String, String>>,)> = self
            .load_stmt
            .query_as()
            .bind(session_id)
            .bind(Utc::now())
            .fetch_optional(&mut conn)
            .await?;
        Ok(res.map(|(value,)| value.0))
    }

    async fn update_session(
        &self,
        session_id: &str,
        entries: &BTreeMap<String, String>,
        expires: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let expires = match expires {
            Some(expires) => Some(chrono::Duration::from_std(expires)?),
            None => None,
        };

        self.update_stmt
            .query()
            .bind(session_id)
            .bind(Json(entries))
            .bind(expires.map(|expires| Utc::now() + expires))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        self.remove_stmt
            .query()
            .bind(session_id)
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

async fn do_cleanup(pool: &PgPool, stmt: &PgStatement<'static>) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;
    stmt.query().bind(Utc::now()).execute(&mut conn).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness;

    #[tokio::test]
    async fn test() {
        let pool = PgPool::connect("postgres://postgres:123456@localhost/test_poem_sessions")
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        sqlx::query(
            r#"
        create table if not exists poem_sessions (
            id varchar not null primary key,
            expires timestamp with time zone null,
            session jsonb not null
        )
        "#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        sqlx::query(
            r#"
        create index if not exists poem_sessions_expires_idx on poem_sessions (expires)
        "#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        let storage = PgSessionStorage::new(
            DatabaseConfig::new().cleanup_period(Duration::from_secs(1)),
            pool,
        )
        .await
        .unwrap();
        test_harness::test_storage(storage).await;
    }
}
