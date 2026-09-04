//! [`EmuProfile`] — the portable `.emuprofile` recipe (ADR 0004).
//!
//! This is the **wire format users share**, so its serde shape must match
//! `schemas/emuprofile/v1.schema.json` field-for-field. A test in milestone M4 will enforce that
//! against the committed fixtures; for now `parses_every_valid_fixture` covers the happy path.
//!
//! The recipe never carries SDK components or disk images — only references.
//!
//! Known schema/CLI naming drift to reconcile in M4 (tracked here, not silently papered over):
//! - schema image type `android-automotive_playstore` vs the `sdkmanager` tag
//!   `android-automotive-playstore` ([`super::image::ImageType`]).
//! - schema `hardware.graphics` value `hardware` vs the emulator `-gpu` value `host`
//!   ([`super::emulator::Graphics::Host`]).

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::emulator::{Graphics, Hardware};
use crate::model::image::{Abi, ImageCoord, ImageType};

/// Current `.emuprofile` schema version string.
pub const SCHEMA_VERSION: &str = "1.0";

/// A shareable emulator recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmuProfile {
    /// Must equal [`SCHEMA_VERSION`].
    pub schema_version: String,
    /// Free-form profile name (1–100 chars).
    pub name: String,
    /// Always `"android"` — other values are rejected before a plan is produced.
    pub platform: String,
    /// Optional human description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Target device.
    pub device: ProfileDevice,
    /// Target system image.
    pub image: ProfileImage,
    /// Hardware knobs.
    pub hardware: ProfileHardware,
    /// Optional post-create seeding (APK filenames + settings; never binaries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<ProfileSeed>,
}

/// Device section of a profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDevice {
    /// Google device profile id (e.g. `pixel_6`), or `custom` when [`Self::custom`] is set.
    pub profile: String,
    /// Inline definition when `profile == "custom"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<CustomDevice>,
}

/// Inline custom device geometry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomDevice {
    /// Screen width in pixels.
    pub width_px: u32,
    /// Screen height in pixels.
    pub height_px: u32,
    /// Screen density in dpi.
    pub density_dpi: u32,
    /// Physical diagonal in inches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagonal_inches: Option<f32>,
    /// Recommended RAM in MiB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_mb: Option<u32>,
}

/// Image type as spelled in the `.emuprofile` schema enum (see module note on drift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProfileImageType {
    /// `default`
    #[serde(rename = "default")]
    Default,
    /// `google_apis`
    #[serde(rename = "google_apis")]
    GoogleApis,
    /// `google_apis_playstore`
    #[serde(rename = "google_apis_playstore")]
    GoogleApisPlaystore,
    /// `android-wear`
    #[serde(rename = "android-wear")]
    AndroidWear,
    /// `android-tv`
    #[serde(rename = "android-tv")]
    AndroidTv,
    /// `android-automotive`
    #[serde(rename = "android-automotive")]
    AndroidAutomotive,
    /// `android-automotive_playstore` (schema spelling)
    #[serde(rename = "android-automotive_playstore")]
    AndroidAutomotivePlaystore,
}

impl From<ProfileImageType> for ImageType {
    fn from(t: ProfileImageType) -> Self {
        match t {
            ProfileImageType::Default => ImageType::Default,
            ProfileImageType::GoogleApis => ImageType::GoogleApis,
            ProfileImageType::GoogleApisPlaystore => ImageType::GoogleApisPlaystore,
            ProfileImageType::AndroidWear => ImageType::AndroidWear,
            ProfileImageType::AndroidTv => ImageType::AndroidTv,
            ProfileImageType::AndroidAutomotive => ImageType::AndroidAutomotive,
            ProfileImageType::AndroidAutomotivePlaystore => ImageType::AndroidAutomotivePlaystore,
        }
    }
}

/// Image section of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileImage {
    /// Android API level (21–100 per schema).
    pub api: u32,
    /// Image type.
    #[serde(rename = "type")]
    pub image_type: ProfileImageType,
    /// CPU ABI (`x86_64` or `arm64-v8a` per schema).
    pub abi: Abi,
}

/// Graphics value as spelled in the `.emuprofile` schema enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProfileGraphics {
    /// `auto`
    #[serde(rename = "auto")]
    Auto,
    /// `hardware` (schema spelling of "use the host GPU")
    #[serde(rename = "hardware")]
    Hardware,
    /// `swiftshader_indirect`
    #[serde(rename = "swiftshader_indirect")]
    SwiftshaderIndirect,
}

impl From<ProfileGraphics> for Graphics {
    fn from(g: ProfileGraphics) -> Self {
        match g {
            ProfileGraphics::Auto => Graphics::Auto,
            ProfileGraphics::Hardware => Graphics::Host,
            ProfileGraphics::SwiftshaderIndirect => Graphics::SwiftshaderIndirect,
        }
    }
}

/// Hardware section of a profile. `ramMb` + `storageGb` required; the rest have schema defaults.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileHardware {
    /// RAM in MiB.
    pub ram_mb: u32,
    /// Internal storage in **GiB** (note the unit differs from [`Hardware::storage_mb`]).
    pub storage_gb: u32,
    /// Density override in dpi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpi: Option<u32>,
    /// Graphics backend.
    #[serde(default = "default_graphics")]
    pub graphics: ProfileGraphics,
    /// Quickboot snapshots enabled.
    #[serde(default = "default_true")]
    pub snapshots: bool,
    /// Force cold boot.
    #[serde(default)]
    pub cold_boot: bool,
    /// Show the device frame.
    #[serde(default = "default_true")]
    pub device_frame: bool,
}

fn default_graphics() -> ProfileGraphics {
    ProfileGraphics::Auto
}
const fn default_true() -> bool {
    true
}

/// Optional seeding section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSeed {
    /// APK **filenames** the importer must supply separately (never embedded).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apks: Vec<String>,
    /// Post-boot settings to apply (locale, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<SeedSettings>,
}

/// Settings applied to the emulator after first boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeedSettings {
    /// Locale tag such as `en-US`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Any other `key -> string value` settings.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, String>,
}

impl EmuProfile {
    /// `true` when this recipe targets Android. A non-android profile never yields a
    /// [`Plan`](crate::model::plan::Plan).
    #[must_use]
    pub fn is_android(&self) -> bool {
        self.platform == "android"
    }

    /// Validate the invariants `emu-core` owns (structure is already enforced by serde +
    /// the JSON Schema at import). Returns the first problem found.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(CoreError::invalid(
                "emuprofile",
                format!(
                    "unsupported schemaVersion `{}` (expected `{SCHEMA_VERSION}`)",
                    self.schema_version
                ),
            ));
        }
        if !self.is_android() {
            return Err(CoreError::invalid(
                "emuprofile",
                format!(
                    "platform `{}` is not supported (android only)",
                    self.platform
                ),
            ));
        }
        if self.device.profile == "custom" && self.device.custom.is_none() {
            return Err(CoreError::invalid(
                "emuprofile",
                "device.profile is `custom` but device.custom is missing",
            ));
        }
        Ok(())
    }

    /// The system-image coordinate this recipe asks for.
    #[must_use]
    pub fn image_coord(&self) -> ImageCoord {
        ImageCoord::new(self.image.api, self.image.image_type.into(), self.image.abi)
    }

    /// The recipe's hardware mapped onto the domain [`Hardware`] type (GiB → MiB).
    #[must_use]
    pub fn hardware(&self) -> Hardware {
        let hw = &self.hardware;
        Hardware {
            ram_mb: hw.ram_mb,
            storage_mb: hw.storage_gb.saturating_mul(1024),
            graphics: hw.graphics.into(),
            snapshots: hw.snapshots,
            cold_boot: hw.cold_boot,
            device_frame: hw.device_frame,
            dpi_override: hw.dpi,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/emuprofile/fixtures/valid"
    );
    const INVALID_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/emuprofile/fixtures/invalid"
    );

    fn read(dir: &str, name: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(dir).join(name))
            .unwrap_or_else(|e| panic!("read {name}: {e}"))
    }

    #[test]
    fn parses_every_valid_fixture() {
        for name in ["minimal.json", "full.json", "custom-device.json"] {
            let raw = read(VALID_DIR, name);
            let profile: EmuProfile =
                serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
            profile
                .validate()
                .unwrap_or_else(|e| panic!("{name} failed validate(): {e}"));
            assert!(profile.is_android());
        }
    }

    #[test]
    fn valid_fixture_round_trips() {
        let raw = read(VALID_DIR, "full.json");
        let profile: EmuProfile = serde_json::from_str(&raw).expect("parse");
        let reserialized = serde_json::to_string(&profile).expect("ser");
        let again: EmuProfile = serde_json::from_str(&reserialized).expect("reparse");
        assert_eq!(profile, again);
    }

    #[test]
    fn rejects_unknown_field_fixture() {
        let raw = read(INVALID_DIR, "unknown-field.json");
        let err = serde_json::from_str::<EmuProfile>(&raw).unwrap_err();
        assert!(err.to_string().contains("unknown field"), "{err}");
    }

    #[test]
    fn rejects_wrong_platform_after_parse() {
        let raw = read(INVALID_DIR, "wrong-platform.json");
        // It may parse structurally; validate() must catch the platform.
        if let Ok(profile) = serde_json::from_str::<EmuProfile>(&raw) {
            let err = profile.validate().unwrap_err();
            assert_eq!(err.code(), "invalid");
        }
    }

    #[test]
    fn maps_units_and_graphics() {
        let raw = read(VALID_DIR, "full.json");
        let profile: EmuProfile = serde_json::from_str(&raw).expect("parse");
        let hw = profile.hardware();
        assert_eq!(hw.storage_mb % 1024, 0, "GiB should convert to whole MiB");
        let _ = profile.image_coord(); // must not panic
    }
}
