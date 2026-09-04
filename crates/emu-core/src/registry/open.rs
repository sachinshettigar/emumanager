use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::{CoreError, Result};

/// Handle to the local SQLite registry.
#[derive(Debug, Clone)]
pub struct Registry {
    pool: SqlitePool,
}

impl Registry {
    /// Open `<data_dir>/db.sqlite`, creating the directory and file if missing,
    /// enabling WAL journalling and foreign keys, then running all pending
    /// migrations. Re-opening the same directory just reconnects.
    pub async fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir).map_err(|e| CoreError::Fs {
            path: data_dir.display().to_string(),
            detail: e.to_string(),
        })?;

        let options = SqliteConnectOptions::new()
            .filename(data_dir.join("db.sqlite"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(|e| CoreError::Db {
                detail: e.to_string(),
            })?;

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .map_err(|e| CoreError::Db {
                detail: e.to_string(),
            })?;

        Ok(Self { pool })
    }

    /// The connection pool, for queries elsewhere in the crate.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
