//! [`Emulator`] — a tracked emulator instance (an AVD plus our metadata).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::model::image::ImageCoord;

/// Our stable identifier for a tracked emulator (a ULID string in practice).
///
/// Distinct from `avd_name`, which is the on-disk AVD directory name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmulatorId(pub String);

impl EmulatorId {
    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EmulatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for EmulatorId {
    fn from(s: String) -> Self {
        EmulatorId(s)
    }
}

/// Emulator graphics backend (`-gpu` flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Graphics {
    /// `-gpu auto` — let the emulator decide.
    Auto,
    /// `-gpu host` — use the host GPU (fastest, needs a display).
    Host,
    /// `-gpu swiftshader_indirect` — software rendering (headless / no GPU).
    SwiftshaderIndirect,
}

/// Hardware configuration baked into the AVD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hardware {
    /// RAM in MiB.
    pub ram_mb: u32,
    /// Userdata/internal-storage partition size in MiB.
    pub storage_mb: u32,
    /// Graphics backend.
    pub graphics: Graphics,
    /// Whether snapshots (quickboot) are enabled.
    pub snapshots: bool,
    /// Force a full cold boot every launch (ignore snapshots).
    pub cold_boot: bool,
    /// Show the device frame/skin around the window.
    pub device_frame: bool,
    /// Screen density override in dpi; `None` uses the device profile default.
    pub dpi_override: Option<u32>,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            ram_mb: 2048,
            storage_mb: 6144,
            graphics: Graphics::Auto,
            snapshots: true,
            cold_boot: false,
            device_frame: true,
            dpi_override: None,
        }
    }
}

/// Where a tracked emulator came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
#[non_exhaustive]
pub enum EmulatorSource {
    /// Created by the user in the app, or adopted from an out-of-band AVD.
    Manual {
        /// `true` when `reconcile()` discovered this AVD rather than the app creating it.
        discovered: bool,
    },
    /// Created by applying a saved profile.
    FromProfile {
        /// The saved profile's id.
        profile_id: String,
    },
    /// Created by importing an external `.emuprofile`.
    Imported {
        /// The imported profile's id.
        profile_id: String,
        /// A human label for where it was imported from.
        origin_label: String,
    },
}

/// Runtime lifecycle state (never persisted; produced by `reconcile()` / launch polling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum RunState {
    /// Not running.
    Stopped,
    /// Process started, `sys.boot_completed` not yet `1`.
    Booting,
    /// Fully booted and reachable over adb.
    Running,
    /// Crashed or failed to boot.
    Error,
}

/// Live, non-persisted facts about an emulator, refreshed on probe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveState {
    /// Which emulator this describes.
    pub id: EmulatorId,
    /// Current lifecycle state.
    pub state: RunState,
    /// adb serial (e.g. `emulator-5554`) when running.
    pub adb_serial: Option<String>,
    /// Emulator gRPC control port when running.
    pub grpc_port: Option<u16>,
    /// OS process id when running.
    pub pid: Option<u32>,
    /// Seconds since the process started, when running.
    pub uptime_secs: Option<u64>,
}

/// A tracked emulator: an AVD on disk plus EmuManager metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Emulator {
    /// Our id.
    pub id: EmulatorId,
    /// The on-disk AVD name (`avdmanager` identifier, no spaces).
    pub avd_name: String,
    /// Human-friendly name shown in the UI.
    pub display_name: String,
    /// The device profile this was built from.
    pub device_profile_id: String,
    /// The system image it runs.
    pub image_coord: ImageCoord,
    /// Hardware configuration.
    pub hardware: Hardware,
    /// Provenance.
    pub source: EmulatorSource,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// User notes.
    pub notes: String,
    /// When the row was created (RFC 3339 on the wire).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the row was last modified (RFC 3339 on the wire).
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::image::{Abi, ImageType};
    use time::macros::datetime;

    fn sample() -> Emulator {
        Emulator {
            id: EmulatorId("01J000000000000000000000AB".into()),
            avd_name: "pixel6_api34".into(),
            display_name: "Pixel 6 · API 34".into(),
            device_profile_id: "pixel_6".into(),
            image_coord: ImageCoord::new(34, ImageType::GoogleApisPlaystore, Abi::Arm64V8a),
            hardware: Hardware::default(),
            source: EmulatorSource::Manual { discovered: false },
            tags: vec!["work".into()],
            notes: String::new(),
            created_at: datetime!(2026-09-05 10:00:00 UTC),
            updated_at: datetime!(2026-09-05 10:00:00 UTC),
        }
    }

    #[test]
    fn emulator_json_round_trips() {
        let e = sample();
        let json = serde_json::to_string(&e).expect("ser");
        let back: Emulator = serde_json::from_str(&json).expect("de");
        assert_eq!(e, back);
    }

    #[test]
    fn timestamps_serialize_as_rfc3339() {
        let json = serde_json::to_value(sample()).expect("ser");
        assert_eq!(json["createdAt"], "2026-09-05T10:00:00Z");
    }

    #[test]
    fn source_is_internally_tagged() {
        let json = serde_json::to_value(EmulatorSource::FromProfile {
            profile_id: "p1".into(),
        })
        .expect("ser");
        assert_eq!(json["kind"], "fromProfile");
        assert_eq!(json["profileId"], "p1");
    }

    #[test]
    fn default_hardware_is_sane() {
        let hw = Hardware::default();
        assert!(hw.ram_mb >= 2048);
        assert!(hw.snapshots);
        assert!(!hw.cold_boot);
    }
}
