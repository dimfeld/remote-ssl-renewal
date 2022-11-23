use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use deadpool_sqlite::{Hook, HookError, HookErrorCause};
use eyre::Result;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::cmd::State;

const MIGRATIONS: [&str; 1] = [include_str!("../migrations/0001-init.sql")];

fn create_migrations() -> Migrations<'static> {
    let items = MIGRATIONS.iter().map(|m| M::up(m)).collect::<Vec<_>>();
    Migrations::new(items)
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    let migrations = create_migrations();
    migrations.to_latest(conn)?;
    Ok(())
}

pub async fn create_db() -> Result<deadpool_sqlite::Pool> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("remote-ssl-renewal");
    std::fs::create_dir_all(&config_dir)?;

    let db_path = config_dir.join("data.sqlite3");

    let pool = deadpool_sqlite::Config::new(db_path)
        .builder(deadpool_sqlite::Runtime::Tokio1)?
        .recycle_timeout(Some(Duration::from_secs(5 * 60)))
        .post_create(Hook::async_fn(move |conn, _| {
            Box::pin(async move {
                conn.interact(move |conn| {
                    conn.pragma_update(None, "journal_mode", "WAL")?;
                    conn.pragma_update(None, "synchronous", "NORMAL")?;
                    Ok(())
                })
                .await
                .map_err(|e| HookError::Abort(HookErrorCause::Message(e.to_string())))?
                .map_err(|e| HookError::Abort(HookErrorCause::Backend(e)))?;
                Ok(())
            })
        }))
        .build()?;

    pool.interact(migrate).await?;

    Ok(pool)
}

#[async_trait]
pub trait PoolExtInteract<F, RETVAL, ERR>
where
    F: (FnOnce(&mut rusqlite::Connection) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: Send + 'static,
{
    async fn interact(&self, f: F) -> Result<RETVAL, ERR>;
}

#[async_trait]
pub trait PoolExtTransaction<F, RETVAL, ERR>
where
    F: (FnOnce(&mut rusqlite::Transaction) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: Send + 'static,
{
    async fn transaction(&self, f: F) -> Result<RETVAL, ERR>;
}

#[async_trait]
impl<F, RETVAL, ERR> PoolExtInteract<F, RETVAL, ERR> for deadpool_sqlite::Pool
where
    F: (FnOnce(&mut rusqlite::Connection) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: From<rusqlite::Error> + From<deadpool_sqlite::PoolError> + Send + 'static,
{
    async fn interact(&self, f: F) -> Result<RETVAL, ERR> {
        let conn = self.get().await?;
        let result = conn.interact(move |conn| f(conn)).await.unwrap()?;
        Ok(result)
    }
}

#[async_trait]
impl<F, RETVAL, ERR> PoolExtTransaction<F, RETVAL, ERR> for deadpool_sqlite::Pool
where
    F: (FnOnce(&mut rusqlite::Transaction) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: From<rusqlite::Error> + From<deadpool_sqlite::PoolError> + Send + 'static,
{
    async fn transaction(&self, f: F) -> Result<RETVAL, ERR> {
        let conn = self.get().await?;
        let result = conn
            .interact(move |conn| {
                let mut tx = conn.transaction()?;
                f(&mut tx)
            })
            .await
            .unwrap()?;
        Ok(result)
    }
}

pub struct DbObject {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub creds: String,
}

impl DbObject {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            provider: row.get(2)?,
            creds: row.get(3)?,
        })
    }
}

pub struct DbObjects {
    pub acme_accounts: Vec<DbObject>,
    pub dns_providers: Vec<DbObject>,
    pub endpoints: Vec<DbObject>,
}

pub async fn get_all_objects(state: &Arc<State>) -> Result<DbObjects> {
    state
        .pool
        .interact(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT id, name, provider, creds FROM acme_accounts ORDER BY name",
            )?;
            let acme_accounts = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;

            let mut stmt = conn.prepare_cached(
                "SELECT id, name, provider, creds FROM dns_providers ORDER BY name",
            )?;
            let dns_providers = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;

            let mut stmt = conn
                .prepare_cached("SELECT id, name, provider, creds FROM endpoints ORDER BY name")?;
            let endpoints = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;

            Ok(DbObjects {
                acme_accounts,
                dns_providers,
                endpoints,
            })
        })
        .await
}
