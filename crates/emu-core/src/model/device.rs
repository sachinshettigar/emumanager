//! [`DeviceProfile`] — a hardware profile the emulator can target.
//!
//! Google's profiles come from `avdmanager list device`; users can define custom ones.

use serde::{Deserialize, Serialize};

/// Physical class of device. Drives which system images and hardware options make sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum FormFactor {
    /// Handset.
    Phone,
    /// Tablet.
    Tablet,
    /// Foldable handset.
    Foldable,
    /// Wear OS.
    Wear,
    /// Android TV.
    Tv,
    /// Android Automotive.
    Automotive,
    /// Desktop / freeform.
    Desktop,
}

/// Screen geometry for a [`DeviceProfile`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    /// Width in pixels.
    pub width_px: u32,
    /// Height in pixels.
    pub height_px: u32,
    /// Density in dots per inch (the `-dpi` bucket, e.g. 420).
    pub density_dpi: u32,
    /// Physical diagonal in inches.
    pub diagonal_in: f32,
}

/// A device hardware profile (Google-provided or user-created).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    /// Stable id, e.g. `pixel_6`. For custom profiles this is a slug of `display_name`.
    pub id: String,
    /// Human name, e.g. `Pixel 6`.
    pub display_name: String,
    /// Manufacturer / OEM label (e.g. `Google`). Empty when unknown.
    pub oem: String,
    /// Device class.
    pub form_factor: FormFactor,
    /// Screen geometry.
    pub screen: Screen,
    /// Manufacturer-recommended RAM in MiB.
    pub default_ram_mb: u32,
    /// Sensor ids the profile advertises (e.g. `accelerometer`, `gyroscope`).
    pub sensors: Vec<String>,
    /// Skin name from the device definition's `<d:skin>` element (e.g. `pixel_6`), if it declares
    /// one. This is the bezel/frame artwork the `emulator` binary draws around the screen when
    /// passed `-skin <name>`; `None` means the device has no dedicated frame.
    pub skin: Option<String>,
    /// `true` when the user created this profile rather than Google.
    pub is_custom: bool,
}
