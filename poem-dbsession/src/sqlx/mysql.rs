use std::{collections::BTreeMap, time::Duration};

use chrono::Utc;
use poem::{session::SessionStorage, Result};
use sqlx::{mysql::MySqlStatement, types::Json, Executor, MySqlPool, Statement};

use crate::{cleanup_task::CleanupTask, DatabaseConfig};

const LOAD_SESSION_SQL: &str = r#"
    select session from {table_name}
        where id = ? and (expires is null or expires > ?)
    "#;

const UPDATE_SESSION_SQL: &str = r#"
    insert into {table_name} (id, session, expires) values (?, ?, ?)
        on duplicate key update
            expires = values(expires),
            session = values(session)
"#;

const REMOVE_SESSION_SQL: &str = r#"
    delete from {table_name} where id = ?
"#;

const CLEANUP_SQL: &str = r#"
    delete from {table_name} where expires < ?
"#;

/// Session storage using Mysql.
///
/// # Create the table for session storage
///
/// ```sql
/// create table if not exists poem_sessions (
///     id varchar(128) not null,
///     expires timestamp(6) null,
///     session text not null,
///     primary key (id),
///     key expires (expires)
/// )
/// engine=innodb
/// default charset=utf8
/// ```
pub struct MysqlSessionStorage {
    pool: MySqlPool,
    _cleanup_task: CleanupTask,
    load_stmt: MySqlStatement<'static>,
    update_stmt: MySqlStatement<'static>,
    remove_stmt: MySqlStatement<'static>,
}

impl MysqlSessionStorage {
    /// Create an [`MysqlSessionStorage`].
    pub async fn try_new(config: DatabaseConfig, pool: MySqlPool) -> Result<Self> {
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
impl SessionStorage for MysqlSessionStorage {
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

async fn do_cleanup(pool: &MySqlPool, stmt: &MySqlStatement<'static>) -> sqlx::Result<()> {
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
        let pool = MySqlPool::connect("mysql://root:123456@localhost/test_poem_sessions")
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        sqlx::query(
            r#"
        create table if not exists poem_sessions (
            id varchar(128) not null,
            expires timestamp(6) null,
            session text not null,
            primary key (id),
            key expires (expires)
        )
        engine=innodb
        default charset=utf8
        "#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        let storage = MysqlSessionStorage::new(
            DatabaseConfig::new().cleanup_period(Duration::from_secs(1)),
            pool,
        )
        .await
        .unwrap();
        test_harness::test_storage(storage).await;
    }
}
