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

    /// Insert a minimal tracked-emulator row (task `0015` — `AndroidProvider::create`).
    ///
    /// Only `id`/`avd_name`/`display_name` are stored: the full schema (image coord, hardware,
    /// source, tags, timestamps beyond `created_at`'s own default) is `migrations/0001_init.sql`'s
    /// M3 work. For now the registry only needs to remember enough to resolve our `id` back to the
    /// on-disk `avd_name` for later `avdmanager`/`adb` calls.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Db`] on any SQL failure, including a duplicate `id` or `avd_name`
    /// (the table's own `UNIQUE` constraint on `avd_name`).
    pub async fn insert_emulator(
        &self,
        id: &str,
        avd_name: &str,
        display_name: &str,
    ) -> Result<()> {
        sqlx::query("INSERT INTO emulators (id, avd_name, display_name) VALUES (?1, ?2, ?3)")
            .bind(id)
            .bind(avd_name)
            .bind(display_name)
            .execute(&self.pool)
            .await
            .map_err(|e| CoreError::Db {
                detail: e.to_string(),
            })?;
        Ok(())
    }

    /// Look up a tracked emulator's `(avd_name, display_name)` by id. `None` if untracked.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Db`] on any SQL failure.
    pub async fn get_emulator(&self, id: &str) -> Result<Option<(String, String)>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT avd_name, display_name FROM emulators WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::Db {
            detail: e.to_string(),
        })?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_look_up_a_minimal_emulator_row() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        registry
            .insert_emulator(
                "01J000000000000000000000AB",
                "pixel6_api34",
                "Pixel 6 · API 34",
            )
            .await
            .expect("insert");

        let row = registry
            .get_emulator("01J000000000000000000000AB")
            .await
            .expect("query")
            .expect("row exists");
        assert_eq!(
            row,
            ("pixel6_api34".to_string(), "Pixel 6 · API 34".to_string())
        );

        assert!(registry
            .get_emulator("nonexistent")
            .await
            .expect("query")
            .is_none());
    }

    #[tokio::test]
    async fn duplicate_avd_name_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");
        registry
            .insert_emulator("id-1", "same_name", "First")
            .await
            .expect("first insert");

        let err = registry
            .insert_emulator("id-2", "same_name", "Second")
            .await
            .unwrap_err();
        assert_eq!(err.code(), "db_error");
    }
}
