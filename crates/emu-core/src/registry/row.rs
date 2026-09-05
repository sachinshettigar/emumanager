//! Typed registry records — what [`Registry`](super::Registry)'s methods take and return, instead
//! of bare column tuples.
//!
//! Compound fields (`Hardware`, `EmulatorSource`, tags) live in JSON text columns
//! (`migrations/0002_registry_m3.sql`); the mapping here is the single place that (de)serializes
//! them. Enum-ish scalars (`RunState`, `ImageCoord`) use their existing `Display`/`FromStr`.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use sqlx::sqlite::SqliteRow;
use sqlx::Row as _;

use crate::model::emulator::{EmulatorId, EmulatorSource, Hardware, RunState};
use crate::model::image::ImageCoord;
use crate::{CoreError, Result};

/// A tracked emulator as stored in the `emulators` table — the full M3 record.
#[derive(Debug, Clone, PartialEq)]
pub struct EmulatorRow {
    /// Our stable id (ULID string).
    pub id: EmulatorId,
    /// On-disk AVD directory name (unique).
    pub avd_name: String,
    /// Human-friendly name shown in the UI.
    pub display_name: String,
    /// Device profile the AVD was built from (empty for an adopted AVD whose profile is unknown).
    pub device_profile_id: String,
    /// System image coordinate; `None` when unset/unparseable (e.g. a freshly adopted AVD).
    pub image_coord: Option<ImageCoord>,
    /// Hardware configuration.
    pub hardware: Hardware,
    /// Provenance.
    pub source: EmulatorSource,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// User notes.
    pub notes: String,
    /// Last known lifecycle state (persisted so a restart shows something before `reconcile()`).
    pub last_state: RunState,
    /// adb serial (`emulator-5554`) from the last probe, when running.
    pub adb_serial: Option<String>,
    /// Emulator gRPC control port from the last probe, when known.
    pub grpc_port: Option<u16>,
    /// OS process id of the emulator this app launched, when it launched it.
    pub pid: Option<u32>,
    /// When the row was created.
    pub created_at: OffsetDateTime,
    /// When the row was last modified.
    pub updated_at: OffsetDateTime,
    /// When the emulator was last launched, when known.
    pub launched_at: Option<OffsetDateTime>,
}

impl EmulatorRow {
    /// A new row for a freshly created emulator: `Stopped`, no live-run fields, timestamps `now`.
    #[must_use]
    pub fn new(
        id: EmulatorId,
        avd_name: String,
        display_name: String,
        device_profile_id: String,
        image_coord: ImageCoord,
        hardware: Hardware,
        source: EmulatorSource,
        now: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            avd_name,
            display_name,
            device_profile_id,
            image_coord: Some(image_coord),
            hardware,
            source,
            tags: Vec::new(),
            notes: String::new(),
            last_state: RunState::Stopped,
            adb_serial: None,
            grpc_port: None,
            pid: None,
            created_at: now,
            updated_at: now,
            launched_at: None,
        }
    }

    /// Map one `SELECT * FROM emulators` row.
    pub(super) fn from_sqlite_row(row: &SqliteRow) -> Result<Self> {
        let id: String = try_get(row, "id")?;
        let hardware_json: String = try_get(row, "hardware_json")?;
        let source_json: String = try_get(row, "source_json")?;
        let tags_json: String = try_get(row, "tags_json")?;
        let image_coord: String = try_get(row, "image_coord")?;
        let last_state: String = try_get(row, "last_state")?;
        let grpc_port: Option<i64> = try_get(row, "grpc_port")?;
        let pid: Option<i64> = try_get(row, "pid")?;

        Ok(Self {
            id: EmulatorId(id),
            avd_name: try_get(row, "avd_name")?,
            display_name: try_get(row, "display_name")?,
            device_profile_id: try_get(row, "device_profile_id")?,
            image_coord: if image_coord.is_empty() {
                None
            } else {
                image_coord.parse().ok()
            },
            hardware: serde_json::from_str(&hardware_json)
                .map_err(|e| CoreError::parse("registry hardware_json", e))?,
            source: serde_json::from_str(&source_json)
                .map_err(|e| CoreError::parse("registry source_json", e))?,
            tags: serde_json::from_str(&tags_json)
                .map_err(|e| CoreError::parse("registry tags_json", e))?,
            notes: try_get(row, "notes")?,
            last_state: parse_run_state(&last_state)?,
            adb_serial: try_get(row, "adb_serial")?,
            grpc_port: grpc_port.and_then(|n| u16::try_from(n).ok()),
            pid: pid.and_then(|n| u32::try_from(n).ok()),
            created_at: parse_ts(&try_get::<String>(row, "created_at")?)?,
            updated_at: parse_ts(&try_get::<String>(row, "updated_at")?)?,
            launched_at: try_get::<Option<String>>(row, "launched_at")?
                .map(|s| parse_ts(&s))
                .transpose()?,
        })
    }

    /// The `hardware` field as the JSON stored in `hardware_json`.
    pub(super) fn hardware_json(&self) -> String {
        serde_json::to_string(&self.hardware).expect("Hardware serializes")
    }

    /// The `source` field as the JSON stored in `source_json`.
    pub(super) fn source_json(&self) -> String {
        serde_json::to_string(&self.source).expect("EmulatorSource serializes")
    }

    /// The `tags` field as the JSON stored in `tags_json`.
    pub(super) fn tags_json(&self) -> String {
        serde_json::to_string(&self.tags).expect("Vec<String> serializes")
    }

    /// `image_coord` as the string stored in the `image_coord` column (`""` for `None`).
    pub(super) fn image_coord_str(&self) -> String {
        self.image_coord
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default()
    }
}

/// One host-readiness probe result (M5 populates these; the row type exists now so the schema and
/// API are complete).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSnapshotRow {
    /// Snapshot id (ULID string).
    pub id: String,
    /// When the probe ran.
    pub captured_at: OffsetDateTime,
    /// Serialized `HostReport`.
    pub report_json: String,
}

impl HostSnapshotRow {
    pub(super) fn from_sqlite_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: try_get(row, "id")?,
            captured_at: parse_ts(&try_get::<String>(row, "captured_at")?)?,
            report_json: try_get(row, "report_json")?,
        })
    }
}

/// `RunState` as the lowercase string stored in `last_state` — matches the enum's own
/// `serde(rename_all = "kebab-case")` (all single words, so kebab == lowercase here).
#[must_use]
pub(super) fn run_state_str(state: RunState) -> &'static str {
    match state {
        RunState::Stopped => "stopped",
        RunState::Booting => "booting",
        RunState::Running => "running",
        RunState::Error => "error",
    }
}

fn parse_run_state(s: &str) -> Result<RunState> {
    match s {
        "stopped" => Ok(RunState::Stopped),
        "booting" => Ok(RunState::Booting),
        "running" => Ok(RunState::Running),
        "error" => Ok(RunState::Error),
        other => Err(CoreError::parse(
            "registry last_state",
            format!("unknown run state {other:?}"),
        )),
    }
}

/// SQLite `strftime('%Y-%m-%dT%H:%M:%fZ', ...)` output is RFC 3339 (fractional seconds + `Z`).
pub(super) fn format_ts(ts: OffsetDateTime) -> String {
    ts.format(&Rfc3339)
        .expect("OffsetDateTime formats as RFC 3339")
}

fn parse_ts(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| CoreError::parse("registry timestamp", format!("{s:?}: {e}")))
}

fn try_get<'r, T>(row: &'r SqliteRow, col: &'static str) -> Result<T>
where
    T: sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get(col).map_err(|e| CoreError::Db {
        detail: format!("column {col}: {e}"),
    })
}
