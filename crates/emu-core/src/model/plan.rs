//! [`Plan`] — the result of resolving an [`EmuProfile`](crate::model::profile::EmuProfile)
//! against the local install state: what must be downloaded, then what to create.

use serde::{Deserialize, Serialize};

use crate::model::emulator::Hardware;
use crate::model::image::ImageCoord;

/// A category of thing a profile needs present before an emulator can be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum RequirementKind {
    /// The `cmdline-tools` package.
    CmdlineTools,
    /// `platform-tools` (adb).
    PlatformTools,
    /// The `emulator` package.
    Emulator,
    /// A specific system image.
    SystemImage,
    /// The `platforms;android-NN` package matching the image API.
    Platform,
}

/// Whether a [`Requirement`] is already satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
#[non_exhaustive]
pub enum RequirementStatus {
    /// Already installed locally.
    Present,
    /// Must be downloaded; `sizeBytes` is the archive size when known.
    NeedsDownload {
        /// Download size in bytes, or `None` if the repository didn't say.
        size_bytes: Option<u64>,
    },
}

/// One line of a resolution diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// What kind of component this is.
    pub kind: RequirementKind,
    /// Present or needs-download.
    pub status: RequirementStatus,
    /// The image coordinate, when `kind == SystemImage`.
    pub coord: Option<ImageCoord>,
    /// Human label for the UI, e.g. `"System image · Android 14 (Play Store, arm64)"`.
    pub label: String,
}

impl Requirement {
    /// Bytes this requirement will download (0 when already present or size unknown).
    #[must_use]
    pub fn download_bytes(&self) -> u64 {
        match self.status {
            RequirementStatus::NeedsDownload {
                size_bytes: Some(n),
            } => n,
            _ => 0,
        }
    }
}

/// Everything needed to create one AVD once its requirements are satisfied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSpec {
    /// Proposed on-disk AVD name (unique).
    pub avd_name: String,
    /// Display name for the tracked instance.
    pub display_name: String,
    /// Device profile id to build on.
    pub device_profile_id: String,
    /// System image to use.
    pub image_coord: ImageCoord,
    /// Hardware configuration.
    pub hardware: Hardware,
}

/// The plan produced by resolving a profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// One entry per component the profile depends on.
    pub diff: Vec<Requirement>,
    /// What to create after the downloads finish.
    pub create_spec: CreateSpec,
}

impl Plan {
    /// Total bytes to download to satisfy this plan.
    #[must_use]
    pub fn total_download_bytes(&self) -> u64 {
        self.diff.iter().map(Requirement::download_bytes).sum()
    }

    /// `true` when nothing needs downloading — the emulator can be created immediately.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.diff
            .iter()
            .all(|r| matches!(r.status, RequirementStatus::Present))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::image::{Abi, ImageType};

    fn spec() -> CreateSpec {
        CreateSpec {
            avd_name: "pixel6_api34".into(),
            display_name: "Pixel 6".into(),
            device_profile_id: "pixel_6".into(),
            image_coord: ImageCoord::new(34, ImageType::GoogleApis, Abi::X86_64),
            hardware: Hardware::default(),
        }
    }

    #[test]
    fn totals_and_readiness() {
        let plan = Plan {
            diff: vec![
                Requirement {
                    kind: RequirementKind::PlatformTools,
                    status: RequirementStatus::Present,
                    coord: None,
                    label: "platform-tools".into(),
                },
                Requirement {
                    kind: RequirementKind::SystemImage,
                    status: RequirementStatus::NeedsDownload {
                        size_bytes: Some(1_200_000_000),
                    },
                    coord: Some(ImageCoord::new(34, ImageType::GoogleApis, Abi::X86_64)),
                    label: "System image".into(),
                },
            ],
            create_spec: spec(),
        };
        assert_eq!(plan.total_download_bytes(), 1_200_000_000);
        assert!(!plan.is_ready());
    }

    #[test]
    fn ready_plan_has_no_downloads() {
        let plan = Plan {
            diff: vec![Requirement {
                kind: RequirementKind::Emulator,
                status: RequirementStatus::Present,
                coord: None,
                label: "emulator".into(),
            }],
            create_spec: spec(),
        };
        assert!(plan.is_ready());
        assert_eq!(plan.total_download_bytes(), 0);
    }

    #[test]
    fn requirement_status_is_tagged() {
        let json = serde_json::to_value(RequirementStatus::NeedsDownload {
            size_bytes: Some(10),
        })
        .expect("ser");
        assert_eq!(json["status"], "needsDownload");
        assert_eq!(json["sizeBytes"], 10);
    }
}
