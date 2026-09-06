use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::model::emulator::{EmulatorSource, Hardware, RunState};
use crate::registry::row::{format_ts, run_state_str, EmulatorRow, HostSnapshotRow};
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

    // ---- emulators ----------------------------------------------------------------------------

    /// Insert `row`, or replace every non-`id` column if a row with that `id` already exists.
    /// `created_at` is preserved on update (only set on first insert).
    ///
    /// # Errors
    /// [`CoreError::Db`] on any SQL failure, including a duplicate `avd_name` on a *different* id
    /// (the table's `UNIQUE` constraint).
    pub async fn upsert_emulator(&self, row: &EmulatorRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO emulators (
                 id, avd_name, display_name, device_profile_id, image_coord, hardware_json,
                 source_json, tags_json, notes, last_state, adb_serial, grpc_port, pid,
                 created_at, updated_at, launched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
                 avd_name          = excluded.avd_name,
                 display_name      = excluded.display_name,
                 device_profile_id = excluded.device_profile_id,
                 image_coord       = excluded.image_coord,
                 hardware_json     = excluded.hardware_json,
                 source_json       = excluded.source_json,
                 tags_json         = excluded.tags_json,
                 notes             = excluded.notes,
                 last_state        = excluded.last_state,
                 adb_serial        = excluded.adb_serial,
                 grpc_port         = excluded.grpc_port,
                 pid               = excluded.pid,
                 updated_at        = excluded.updated_at,
                 launched_at       = excluded.launched_at",
        )
        .bind(row.id.as_str())
        .bind(&row.avd_name)
        .bind(&row.display_name)
        .bind(&row.device_profile_id)
        .bind(row.image_coord_str())
        .bind(row.hardware_json())
        .bind(row.source_json())
        .bind(row.tags_json())
        .bind(&row.notes)
        .bind(run_state_str(row.last_state))
        .bind(row.adb_serial.as_deref())
        .bind(row.grpc_port.map(i64::from))
        .bind(row.pid.map(i64::from))
        .bind(format_ts(row.created_at))
        .bind(format_ts(row.updated_at))
        .bind(row.launched_at.map(format_ts))
        .execute(&self.pool)
        .await
        .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// The full record for `id`, or `None` if untracked.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure; [`CoreError::Parse`] if a stored JSON/enum column is corrupt.
    pub async fn get_row(&self, id: &str) -> Result<Option<EmulatorRow>> {
        let row = sqlx::query("SELECT * FROM emulators WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        row.as_ref().map(EmulatorRow::from_sqlite_row).transpose()
    }

    /// Every tracked emulator, newest first.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure; [`CoreError::Parse`] if a stored column is corrupt.
    pub async fn list_rows(&self) -> Result<Vec<EmulatorRow>> {
        let rows = sqlx::query("SELECT * FROM emulators ORDER BY created_at DESC, id DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        rows.iter().map(EmulatorRow::from_sqlite_row).collect()
    }

    /// Overwrite the live-run columns (`last_state`, `adb_serial`, `grpc_port`, `pid`,
    /// `launched_at`) and bump `updated_at`. Used by `launch` and `reconcile()`.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn set_run_fields(
        &self,
        id: &str,
        last_state: RunState,
        adb_serial: Option<&str>,
        grpc_port: Option<u16>,
        pid: Option<u32>,
        launched_at: Option<OffsetDateTime>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE emulators SET
                 last_state = ?2, adb_serial = ?3, grpc_port = ?4, pid = ?5, launched_at = ?6,
                 updated_at = ?7
             WHERE id = ?1",
        )
        .bind(id)
        .bind(run_state_str(last_state))
        .bind(adb_serial)
        .bind(grpc_port.map(i64::from))
        .bind(pid.map(i64::from))
        .bind(launched_at.map(format_ts))
        .bind(format_ts(OffsetDateTime::now_utc()))
        .execute(&self.pool)
        .await
        .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// Change the display name and bump `updated_at`.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn rename(&self, id: &str, display_name: &str) -> Result<()> {
        sqlx::query("UPDATE emulators SET display_name = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id)
            .bind(display_name)
            .bind(format_ts(OffsetDateTime::now_utc()))
            .execute(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// Replace an emulator's provenance (used when applying a profile → `Imported`). Bumps
    /// `updated_at`.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn set_source(&self, id: &str, source: &EmulatorSource) -> Result<()> {
        let json = serde_json::to_string(source).map_err(|e| CoreError::Db {
            detail: format!("serializing source: {e}"),
        })?;
        sqlx::query("UPDATE emulators SET source_json = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id)
            .bind(json)
            .bind(format_ts(OffsetDateTime::now_utc()))
            .execute(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// Replace the stored hardware config and bump `updated_at`.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn set_hardware(&self, id: &str, hardware: &Hardware) -> Result<()> {
        let json = serde_json::to_string(hardware).map_err(|e| CoreError::Db {
            detail: format!("serializing hardware: {e}"),
        })?;
        sqlx::query("UPDATE emulators SET hardware_json = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id)
            .bind(json)
            .bind(format_ts(OffsetDateTime::now_utc()))
            .execute(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// Delete the row for `id`. Returns `true` if a row was removed, `false` if none matched.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn delete_row(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM emulators WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(result.rows_affected() > 0)
    }

    // ---- saved profiles -------------------------------------------------------------------

    /// Insert or replace a saved `.emuprofile` recipe by name.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn save_profile(&self, name: &str, description: &str, json: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO profiles (name, description, json) VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET description = excluded.description, json = excluded.json",
        )
        .bind(name)
        .bind(description)
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// Every saved profile as `(name, description, created_at)`, newest first.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn list_profiles(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT name, description, created_at FROM profiles ORDER BY created_at DESC, name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| db_err(&e))?;
        Ok(rows)
    }

    /// A saved profile's JSON body by name, or `None`.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn get_profile(&self, name: &str) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>("SELECT json FROM profiles WHERE name = ?1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(row.map(|r| r.0))
    }

    /// Delete a saved profile. `true` if a row was removed.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn delete_profile(&self, name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM profiles WHERE name = ?1")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        Ok(result.rows_affected() > 0)
    }

    // ---- host snapshots ---------------------------------------------------------------------

    /// Insert a host-readiness probe result.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn insert_host_snapshot(&self, snapshot: &HostSnapshotRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO host_snapshots (id, captured_at, report_json) VALUES (?1, ?2, ?3)",
        )
        .bind(&snapshot.id)
        .bind(format_ts(snapshot.captured_at))
        .bind(&snapshot.report_json)
        .execute(&self.pool)
        .await
        .map_err(|e| db_err(&e))?;
        Ok(())
    }

    /// The most recent host snapshot, or `None` if the host has never been probed.
    ///
    /// # Errors
    /// [`CoreError::Db`] on SQL failure.
    pub async fn latest_host_snapshot(&self) -> Result<Option<HostSnapshotRow>> {
        let row = sqlx::query("SELECT * FROM host_snapshots ORDER BY captured_at DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| db_err(&e))?;
        row.as_ref()
            .map(HostSnapshotRow::from_sqlite_row)
            .transpose()
    }
}

fn db_err(e: &sqlx::Error) -> CoreError {
    CoreError::Db {
        detail: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::emulator::{EmulatorId, EmulatorSource, Graphics};
    use crate::model::image::{Abi, ImageCoord, ImageType};

    fn sample_row(id: &str, avd: &str) -> EmulatorRow {
        EmulatorRow::new(
            EmulatorId(id.to_string()),
            avd.to_string(),
            format!("{avd} display"),
            "pixel_6".to_string(),
            ImageCoord::new(34, ImageType::GoogleApisPlaystore, Abi::X86_64),
            Hardware::default(),
            EmulatorSource::Manual { discovered: false },
            OffsetDateTime::now_utc(),
        )
    }

    #[tokio::test]
    async fn migration_0002_applies_and_every_column_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        let mut row = sample_row("01J000000000000000000000AB", "pixel6_api34");
        row.tags = vec!["work".into(), "ci".into()];
        row.notes = "scratch avd".into();
        row.hardware.ram_mb = 4096;
        row.hardware.graphics = Graphics::Host;
        registry.upsert_emulator(&row).await.expect("insert");

        let back = registry
            .get_row("01J000000000000000000000AB")
            .await
            .expect("query")
            .expect("row exists");
        assert_eq!(back, row);
        assert_eq!(back.hardware.ram_mb, 4096);
        assert_eq!(back.hardware.graphics, Graphics::Host);
        assert_eq!(back.tags, vec!["work".to_string(), "ci".to_string()]);
        assert_eq!(back.last_state, RunState::Stopped);
        assert!(back.adb_serial.is_none());
    }

    #[tokio::test]
    async fn upsert_replaces_and_preserves_created_at() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        let mut row = sample_row("id-1", "avd_one");
        registry.upsert_emulator(&row).await.expect("insert");
        let created = registry.get_row("id-1").await.unwrap().unwrap().created_at;

        row.display_name = "renamed via upsert".into();
        row.updated_at = OffsetDateTime::now_utc();
        registry.upsert_emulator(&row).await.expect("update");

        let back = registry.get_row("id-1").await.unwrap().unwrap();
        assert_eq!(back.display_name, "renamed via upsert");
        assert_eq!(
            back.created_at, created,
            "created_at is not touched on update"
        );
    }

    #[tokio::test]
    async fn set_run_fields_rename_set_hardware_and_delete() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");
        registry
            .upsert_emulator(&sample_row("id-1", "avd_one"))
            .await
            .expect("insert");

        registry
            .set_run_fields(
                "id-1",
                RunState::Running,
                Some("emulator-5554"),
                Some(8554),
                Some(4242),
                Some(OffsetDateTime::now_utc()),
            )
            .await
            .expect("set_run_fields");
        let back = registry.get_row("id-1").await.unwrap().unwrap();
        assert_eq!(back.last_state, RunState::Running);
        assert_eq!(back.adb_serial.as_deref(), Some("emulator-5554"));
        assert_eq!(back.grpc_port, Some(8554));
        assert_eq!(back.pid, Some(4242));
        assert!(back.launched_at.is_some());

        registry.rename("id-1", "New Name").await.expect("rename");
        assert_eq!(
            registry
                .get_row("id-1")
                .await
                .unwrap()
                .unwrap()
                .display_name,
            "New Name"
        );

        let hw = Hardware {
            storage_mb: 12288,
            ..Hardware::default()
        };
        registry
            .set_hardware("id-1", &hw)
            .await
            .expect("set_hardware");
        assert_eq!(
            registry
                .get_row("id-1")
                .await
                .unwrap()
                .unwrap()
                .hardware
                .storage_mb,
            12288
        );

        assert!(registry.delete_row("id-1").await.expect("delete"));
        assert!(!registry.delete_row("id-1").await.expect("delete again"));
        assert!(registry.get_row("id-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_rows_is_newest_first_and_duplicate_avd_name_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        registry
            .upsert_emulator(&sample_row("id-old", "avd_old"))
            .await
            .expect("insert old");
        registry
            .upsert_emulator(&sample_row("id-new", "avd_new"))
            .await
            .expect("insert new");

        let rows = registry.list_rows().await.expect("list");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].id.as_str(),
            "id-new",
            "created_at DESC, then id DESC"
        );

        // Same avd_name under a different id violates the UNIQUE constraint.
        let clash = sample_row("id-clash", "avd_new");
        let err = registry.upsert_emulator(&clash).await.unwrap_err();
        assert_eq!(err.code(), "db_error");
    }

    #[tokio::test]
    async fn host_snapshot_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        assert!(registry.latest_host_snapshot().await.unwrap().is_none());

        let snap = HostSnapshotRow {
            id: "01J00000000000000000000SNP".into(),
            captured_at: OffsetDateTime::now_utc(),
            report_json: r#"{"verdict":"canRun"}"#.into(),
        };
        registry.insert_host_snapshot(&snap).await.expect("insert");

        let back = registry.latest_host_snapshot().await.unwrap().unwrap();
        assert_eq!(back.id, snap.id);
        assert_eq!(back.report_json, snap.report_json);
    }

    #[tokio::test]
    async fn saved_profiles_crud_and_set_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open");

        assert!(registry.list_profiles().await.unwrap().is_empty());
        registry
            .save_profile(
                "QA baseline",
                "checkout regressions",
                r#"{"schemaVersion":"1.0"}"#,
            )
            .await
            .expect("save");
        registry
            .save_profile(
                "QA baseline",
                "updated",
                r#"{"schemaVersion":"1.0","name":"x"}"#,
            )
            .await
            .expect("upsert");

        let list = registry.list_profiles().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "QA baseline");
        assert_eq!(list[0].1, "updated");
        assert!(registry
            .get_profile("QA baseline")
            .await
            .unwrap()
            .unwrap()
            .contains("\"name\":\"x\""));
        assert!(registry.delete_profile("QA baseline").await.unwrap());
        assert!(!registry.delete_profile("QA baseline").await.unwrap());

        // set_source
        registry
            .upsert_emulator(&sample_row("id-1", "avd_one"))
            .await
            .expect("insert emulator");
        registry
            .set_source(
                "id-1",
                &EmulatorSource::Imported {
                    profile_id: "QA baseline".into(),
                    origin_label: "imported .emuprofile".into(),
                },
            )
            .await
            .expect("set_source");
        let row = registry.get_row("id-1").await.unwrap().unwrap();
        assert!(matches!(row.source, EmulatorSource::Imported { .. }));
    }
}
