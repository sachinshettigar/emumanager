//! Parses Google's per-tag system-image manifests (`sys-img2-3.xml`) into [`SysImgEntry`]s the
//! Create wizard's image picker needs.
//!
//! Source and shape cited against real, trimmed captures in `tests/fixtures/sys-img2-3-*.xml`
//! (captured 2026-09-05). Unlike `catalog::parse` (task `0010`), system images are **not** listed
//! in the main `repository2-3.xml` — they live in one manifest per Google "tag" family, at the
//! URLs in [`MANIFEST_URLS`] (curl-verified 2026-09-05). A `<remotePackage path="...">` whose
//! path doesn't round-trip through [`ImageCoord::from_str`] — an extension-level package like
//! `system-images;android-34-ext12;google_apis_playstore;arm64-v8a` (the real feed publishes many
//! of these), or any tag/abi our closed enums don't cover — is skipped, not an error: callers only
//! care about the plain coordinates `avdmanager create avd -k ...` can actually target.

use emu_core::model::{ImageCoord, SystemImage};
use emu_core::{CoreError, Result};

/// Every real manifest URL this parser is meant to be fed, one per Google "tag" family.
/// `android-automotive-playstore` has no manifest of its own — those packages ship inside the
/// `android-automotive` manifest, tagged accordingly (verified 2026-09-05).
pub const MANIFEST_URLS: &[&str] = &[
    "https://dl.google.com/android/repository/sys-img/android/sys-img2-3.xml",
    "https://dl.google.com/android/repository/sys-img/google_apis/sys-img2-3.xml",
    "https://dl.google.com/android/repository/sys-img/google_apis_playstore/sys-img2-3.xml",
    "https://dl.google.com/android/repository/sys-img/android-wear/sys-img2-3.xml",
    "https://dl.google.com/android/repository/sys-img/android-tv/sys-img2-3.xml",
    "https://dl.google.com/android/repository/sys-img/android-automotive/sys-img2-3.xml",
];

/// One system-image package resolved from a manifest, before local install state is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysImgEntry {
    /// The image's identity.
    pub coord: ImageCoord,
    /// `sdkmanager` package revision (`<revision><major>[.<minor>[.<micro>]]`).
    pub revision: String,
    /// Archive size in bytes, when the manifest publishes one.
    pub size_bytes: Option<u64>,
    /// Lower-case hex SHA-1 of the archive, when published.
    pub sha1: Option<String>,
}

/// Parse one `sys-img2-3.xml` manifest. Never errors on a single unrecognized package (see the
/// module doc) — only on XML that doesn't parse at all.
///
/// # Errors
///
/// Returns [`CoreError::Parse`] when `xml` isn't valid UTF-8 or isn't well-formed XML.
pub fn parse(xml: &[u8]) -> Result<Vec<SysImgEntry>> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| CoreError::parse("system image manifest", format!("not valid utf-8: {e}")))?;
    let doc = roxmltree::Document::parse(text)
        .map_err(|e| CoreError::parse("system image manifest", e))?;

    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name("remotePackage"))
        .filter_map(|node| {
            let path = node.attribute("path")?;
            let coord: ImageCoord = path.parse().ok()?;
            let revision = revision_string(&node).unwrap_or_else(|| "0".to_string());
            let (size_bytes, sha1) = archive_facts(&node);
            Some(SysImgEntry {
                coord,
                revision,
                size_bytes,
                sha1,
            })
        })
        .collect())
}

/// Turn a resolved manifest entry into the UI-facing [`SystemImage`], filling in the marketing
/// Android-version string. `installed` must come from the caller's own local-dir knowledge (this
/// module only ever reads a downloaded manifest, never the filesystem).
#[must_use]
pub fn to_system_image(entry: &SysImgEntry, installed: bool) -> SystemImage {
    SystemImage {
        coord: entry.coord,
        android_version: android_version_name(entry.coord.api),
        revision: entry.revision.clone(),
        download_size_bytes: entry.size_bytes,
        installed,
    }
}

/// Marketing Android version for an API level, cited from Android's own API-level reference
/// (<https://developer.android.com/tools/releases/platforms>, checked 2026-09-05). An API level
/// newer than this table (a not-yet-catalogued release) falls back to the level itself so the UI
/// still shows something rather than an empty string.
#[must_use]
pub fn android_version_name(api: u32) -> String {
    let name = match api {
        35 => "15",
        34 => "14",
        33 => "13",
        32 | 31 => "12",
        30 => "11",
        29 => "10",
        28 => "9",
        27 => "8.1",
        26 => "8.0",
        25 => "7.1",
        24 => "7.0",
        23 => "6.0",
        22 => "5.1",
        21 => "5.0",
        _ => return api.to_string(),
    };
    name.to_string()
}

/// Text content of the first direct child named `tag`, if any.
fn text_of<'a>(node: &roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children().find(|n| n.has_tag_name(tag))?.text()
}

/// Join `<major>[.<minor>[.<micro>]]` with `.` — same shape as `catalog::parse`'s revisions.
fn revision_string(package: &roxmltree::Node<'_, '_>) -> Option<String> {
    let revision = package.children().find(|n| n.has_tag_name("revision"))?;
    let major = text_of(&revision, "major")?;
    let mut parts = vec![major];
    if let Some(minor) = text_of(&revision, "minor") {
        parts.push(minor);
        if let Some(micro) = text_of(&revision, "micro") {
            parts.push(micro);
        }
    }
    Some(parts.join("."))
}

/// `(size_bytes, sha1)` from the package's first `<archive><complete>`, when present.
fn archive_facts(package: &roxmltree::Node<'_, '_>) -> (Option<u64>, Option<String>) {
    let Some(complete) = package
        .descendants()
        .find(|n| n.has_tag_name("archive"))
        .and_then(|a| a.children().find(|n| n.has_tag_name("complete")))
    else {
        return (None, None);
    };
    let size_bytes = text_of(&complete, "size").and_then(|s| s.parse::<u64>().ok());
    let sha1 = complete
        .children()
        .find(|n| n.has_tag_name("checksum") && n.attribute("type") == Some("sha1"))
        .and_then(|n| n.text())
        .map(str::to_string);
    (size_bytes, sha1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use emu_core::model::{Abi, ImageType};

    const PLAYSTORE: &[u8] =
        include_bytes!("../tests/fixtures/sys-img2-3-google-apis-playstore.xml");
    const DEFAULT: &[u8] = include_bytes!("../tests/fixtures/sys-img2-3-android.xml");
    const WEAR: &[u8] = include_bytes!("../tests/fixtures/sys-img2-3-wear.xml");

    #[test]
    fn resolves_playstore_entries_and_skips_extension_level() {
        let got = parse(PLAYSTORE).expect("parse");
        // 3 <remotePackage> in the fixture, one of them extension-level — must be skipped.
        assert_eq!(got.len(), 2);

        let arm = got
            .iter()
            .find(|e| e.coord.abi == Abi::Arm64V8a)
            .expect("arm entry");
        assert_eq!(arm.coord.api, 34);
        assert_eq!(arm.coord.image_type, ImageType::GoogleApisPlaystore);
        assert_eq!(arm.revision, "14");
        assert_eq!(arm.size_bytes, Some(1_548_905_381));
        assert_eq!(
            arm.sha1.as_deref(),
            Some("c307c3301dc52635ebc78b943c39b3c377856ebc")
        );

        assert!(got.iter().any(|e| e.coord.abi == Abi::X86_64));
        assert!(got.iter().all(|e| e.coord.to_string()
            != "system-images;android-34-ext12;google_apis_playstore;arm64-v8a"));
    }

    #[test]
    fn resolves_default_tag() {
        let got = parse(DEFAULT).expect("parse");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].coord.image_type, ImageType::Default);
        assert_eq!(got[0].coord.abi, Abi::X86_64);
    }

    #[test]
    fn resolves_wear_tag() {
        let got = parse(WEAR).expect("parse");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].coord.image_type, ImageType::AndroidWear);
    }

    #[test]
    fn to_system_image_fills_marketing_version_and_installed_flag() {
        let entry = SysImgEntry {
            coord: ImageCoord::new(34, ImageType::GoogleApisPlaystore, Abi::Arm64V8a),
            revision: "14".into(),
            size_bytes: Some(100),
            sha1: Some("abc".into()),
        };
        let img = to_system_image(&entry, true);
        assert_eq!(img.android_version, "14");
        assert!(img.installed);
        assert_eq!(img.download_size_bytes, Some(100));
    }

    #[test]
    fn android_version_name_falls_back_to_api_level_for_unknown() {
        assert_eq!(android_version_name(34), "14");
        assert_eq!(android_version_name(999), "999");
    }

    #[test]
    fn empty_manifest_is_an_empty_list_not_an_error() {
        let empty = b"<sys-img:sdk-sys-img xmlns:sys-img=\"http://schemas.android.com/sdk/android/repo/sys-img2/03\"/>";
        assert_eq!(parse(empty).expect("parse"), vec![]);
    }

    #[test]
    fn malformed_xml_is_a_parse_error_not_a_panic() {
        let err = parse(b"<not-xml").unwrap_err();
        assert_eq!(err.code(), "parse_error");
    }
}
