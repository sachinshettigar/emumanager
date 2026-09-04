//! System images and their coordinates.
//!
//! An image is identified by its [`ImageCoord`], which renders to exactly the
//! `sdkmanager` package path:
//!
//! ```text
//! system-images;android-34;google_apis_playstore;x86_64
//! ```
//!
//! The tag strings (`google_apis_playstore`, `x86_64`, …) are load-bearing — they are what
//! `sdkmanager`/`avdmanager` expect on the command line and emit in `--list` output. They are
//! cited against captured fixtures in `crates/emu-android/tests/fixtures/` (see `AGENTS.md` §6).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Literal first segment of every system-image package path.
const PREFIX: &str = "system-images";

/// The "type" segment of an image coordinate — which API surface the image ships.
///
/// Values are the exact `sdkmanager` tag ids.
///
/// The `#[serde(rename)]` on each variant is kept identical to [`ImageType::tag`] so the wire
/// form and the CLI form never diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImageType {
    /// `default` — AOSP, no Google apps.
    #[serde(rename = "default")]
    Default,
    /// `google_apis` — Google APIs, no Play Store.
    #[serde(rename = "google_apis")]
    GoogleApis,
    /// `google_apis_playstore` — Google APIs + Play Store (updatable, no root).
    #[serde(rename = "google_apis_playstore")]
    GoogleApisPlaystore,
    /// `android-wear` — Wear OS.
    #[serde(rename = "android-wear")]
    AndroidWear,
    /// `android-tv` — Android TV.
    #[serde(rename = "android-tv")]
    AndroidTv,
    /// `android-automotive` — Android Automotive OS.
    #[serde(rename = "android-automotive")]
    AndroidAutomotive,
    /// `android-automotive-playstore` — Automotive with Play Store.
    #[serde(rename = "android-automotive-playstore")]
    AndroidAutomotivePlaystore,
}

impl ImageType {
    /// The `sdkmanager` tag string.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            ImageType::Default => "default",
            ImageType::GoogleApis => "google_apis",
            ImageType::GoogleApisPlaystore => "google_apis_playstore",
            ImageType::AndroidWear => "android-wear",
            ImageType::AndroidTv => "android-tv",
            ImageType::AndroidAutomotive => "android-automotive",
            ImageType::AndroidAutomotivePlaystore => "android-automotive-playstore",
        }
    }

    /// `true` if the image bundles the Play Store (blocks root, allows in-emulator updates).
    #[must_use]
    pub const fn has_play_store(self) -> bool {
        matches!(
            self,
            ImageType::GoogleApisPlaystore | ImageType::AndroidAutomotivePlaystore
        )
    }
}

impl fmt::Display for ImageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

impl FromStr for ImageType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(ImageType::Default),
            "google_apis" => Ok(ImageType::GoogleApis),
            "google_apis_playstore" => Ok(ImageType::GoogleApisPlaystore),
            "android-wear" => Ok(ImageType::AndroidWear),
            "android-tv" => Ok(ImageType::AndroidTv),
            "android-automotive" => Ok(ImageType::AndroidAutomotive),
            "android-automotive-playstore" => Ok(ImageType::AndroidAutomotivePlaystore),
            other => Err(CoreError::parse(
                "image type",
                format!("unknown tag `{other}`"),
            )),
        }
    }
}

/// CPU architecture of a system image / AVD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Abi {
    /// `x86_64` — 64-bit Intel; needs HAXM/WHPX/KVM/AEHD on x86 hosts.
    #[serde(rename = "x86_64")]
    X86_64,
    /// `arm64-v8a` — 64-bit ARM; near-native on Apple Silicon, slow elsewhere.
    #[serde(rename = "arm64-v8a")]
    Arm64V8a,
    /// `x86` — 32-bit Intel (legacy).
    #[serde(rename = "x86")]
    X86,
    /// `armeabi-v7a` — 32-bit ARM (legacy).
    #[serde(rename = "armeabi-v7a")]
    ArmeabiV7a,
}

impl Abi {
    /// The `sdkmanager` / `avdmanager` abi string.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Abi::X86_64 => "x86_64",
            Abi::Arm64V8a => "arm64-v8a",
            Abi::X86 => "x86",
            Abi::ArmeabiV7a => "armeabi-v7a",
        }
    }

    /// `true` for 64-bit ABIs (32-bit images are deprecated on modern API levels).
    #[must_use]
    pub const fn is_64_bit(self) -> bool {
        matches!(self, Abi::X86_64 | Abi::Arm64V8a)
    }
}

impl fmt::Display for Abi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

impl FromStr for Abi {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64" => Ok(Abi::X86_64),
            "arm64-v8a" => Ok(Abi::Arm64V8a),
            "x86" => Ok(Abi::X86),
            "armeabi-v7a" => Ok(Abi::ArmeabiV7a),
            other => Err(CoreError::parse("abi", format!("unknown abi `{other}`"))),
        }
    }
}

/// The identity of a system image: API level + type + ABI.
///
/// `Display`/`FromStr` round-trip the `sdkmanager` package path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCoord {
    /// Android API level, e.g. `34`.
    pub api: u32,
    /// Which API surface the image ships.
    pub image_type: ImageType,
    /// CPU architecture.
    pub abi: Abi,
}

impl ImageCoord {
    /// Construct a coordinate.
    #[must_use]
    pub const fn new(api: u32, image_type: ImageType, abi: Abi) -> Self {
        Self {
            api,
            image_type,
            abi,
        }
    }
}

impl fmt::Display for ImageCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{PREFIX};android-{};{};{}",
            self.api, self.image_type, self.abi
        )
    }
}

impl FromStr for ImageCoord {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(';').collect();
        let [prefix, level_seg, kind_seg, arch_seg] = parts.as_slice() else {
            return Err(CoreError::parse(
                "image coordinate",
                format!("expected 4 `;`-separated segments, got {}", parts.len()),
            ));
        };
        if *prefix != PREFIX {
            return Err(CoreError::parse(
                "image coordinate",
                format!("first segment must be `{PREFIX}`, got `{prefix}`"),
            ));
        }
        let level_str = level_seg.strip_prefix("android-").ok_or_else(|| {
            CoreError::parse(
                "image coordinate",
                format!("second segment must be `android-<api>`, got `{level_seg}`"),
            )
        })?;
        let api: u32 = level_str
            .parse()
            .map_err(|e| CoreError::parse("image coordinate", format!("bad API level: {e}")))?;
        let image_type: ImageType = kind_seg.parse()?;
        let abi: Abi = arch_seg.parse()?;
        Ok(ImageCoord {
            api,
            image_type,
            abi,
        })
    }
}

/// A system image as reported by `sdkmanager` plus local install state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemImage {
    /// The identity of the image.
    pub coord: ImageCoord,
    /// Marketing Android version, e.g. `"14"`.
    pub android_version: String,
    /// Package revision (`sdkmanager` "Version" column), e.g. `"12"`.
    pub revision: String,
    /// Archive download size in bytes, when known.
    pub download_size_bytes: Option<u64>,
    /// Whether the image is present in the managed SDK directory. Derived from the filesystem at
    /// read time — never trusted from cache (`docs/context/domain-model.md` invariants).
    pub installed: bool,
}

/// Filter passed to `Provider::list_images`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFilter {
    /// Restrict to a single API level.
    pub api: Option<u32>,
    /// Restrict to a single image type.
    pub image_type: Option<ImageType>,
    /// Restrict to a single ABI (usually the host-launchable one).
    pub abi: Option<Abi>,
    /// When `true`, only images already installed locally.
    pub installed_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "system-images;android-34;google_apis_playstore;x86_64";

    #[test]
    fn coord_round_trips_through_display_and_fromstr() {
        let coord: ImageCoord = SAMPLE.parse().expect("parse");
        assert_eq!(coord.api, 34);
        assert_eq!(coord.image_type, ImageType::GoogleApisPlaystore);
        assert_eq!(coord.abi, Abi::X86_64);
        assert_eq!(coord.to_string(), SAMPLE);
    }

    #[test]
    fn coord_round_trips_for_every_type_and_abi() {
        let types = [
            ImageType::Default,
            ImageType::GoogleApis,
            ImageType::GoogleApisPlaystore,
            ImageType::AndroidWear,
            ImageType::AndroidTv,
            ImageType::AndroidAutomotive,
            ImageType::AndroidAutomotivePlaystore,
        ];
        let abis = [Abi::X86_64, Abi::Arm64V8a, Abi::X86, Abi::ArmeabiV7a];
        for t in types {
            for a in abis {
                let c = ImageCoord::new(30, t, a);
                let s = c.to_string();
                assert_eq!(s.parse::<ImageCoord>().expect("reparse"), c, "{s}");
            }
        }
    }

    #[test]
    fn rejects_wrong_segment_count() {
        let err = "system-images;android-34;x86_64"
            .parse::<ImageCoord>()
            .unwrap_err();
        assert_eq!(err.code(), "parse_error");
        assert!(err.to_string().contains("4 `;`"));
    }

    #[test]
    fn rejects_bad_prefix() {
        let err = "sysimages;android-34;default;x86_64"
            .parse::<ImageCoord>()
            .unwrap_err();
        assert!(err.to_string().contains("system-images"));
    }

    #[test]
    fn rejects_missing_android_prefix() {
        let err = "system-images;34;default;x86_64"
            .parse::<ImageCoord>()
            .unwrap_err();
        assert!(err.to_string().contains("android-<api>"));
    }

    #[test]
    fn rejects_non_numeric_api() {
        let err = "system-images;android-vanilla;default;x86_64"
            .parse::<ImageCoord>()
            .unwrap_err();
        assert!(err.to_string().contains("API level"));
    }

    #[test]
    fn rejects_unknown_type_and_abi() {
        assert_eq!(
            "system-images;android-34;super_apis;x86_64"
                .parse::<ImageCoord>()
                .unwrap_err()
                .code(),
            "parse_error",
        );
        assert_eq!(
            "system-images;android-34;default;risc-v"
                .parse::<ImageCoord>()
                .unwrap_err()
                .code(),
            "parse_error",
        );
    }

    #[test]
    fn play_store_and_bitness_helpers() {
        assert!(ImageType::GoogleApisPlaystore.has_play_store());
        assert!(!ImageType::GoogleApis.has_play_store());
        assert!(Abi::Arm64V8a.is_64_bit());
        assert!(!Abi::ArmeabiV7a.is_64_bit());
    }

    #[test]
    fn serde_uses_wire_tags() {
        let json = serde_json::to_string(&ImageType::GoogleApisPlaystore).expect("ser");
        assert_eq!(json, "\"google_apis_playstore\"");
        let abi: Abi = serde_json::from_str("\"arm64-v8a\"").expect("de");
        assert_eq!(abi, Abi::Arm64V8a);
    }
}
