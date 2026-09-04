//! Parses Google's Android SDK repository manifest (`repository2-3.xml`) into the
//! [`Component`]s milestone M1 needs: `cmdline-tools;latest`, `platform-tools`, `emulator`.
//!
//! Source and shape cited against a real, trimmed capture: `tests/fixtures/repository2-3.xml`
//! (captured 2026-09-05 from <https://dl.google.com/android/repository/repository2-3.xml>). The
//! schema is Google's `sdk-repository` v3 (`xmlns:sdk=".../repository2/03"`) — undocumented
//! outside the XSDs Android Studio ships, so every element/attribute this parser reads is backed
//! by that captured fixture, not by guesswork.
//!
//! Shape, per `<remotePackage path="...">`:
//! - `<revision><major/><minor/><micro/></revision>` — `<minor>`/`<micro>` are optional; join
//!   whatever is present with `.` (`"37.0.1"`, or `"23.0"` when there is no `<micro>`).
//! - `<archives><archive>...` — one per host build. `<host-os>` is always present; `<host-arch>`
//!   is present only when the OS ships arch-specific builds (macOS always does; Linux/Windows
//!   only do for `emulator`) — its absence means "every arch on this OS".
//! - Inside `<archive><complete>`: `<size>` (bytes), `<checksum type="sha1">`, `<url>` (a bare
//!   filename, relative to [`BASE_URL`] — Google's manifest never publishes SHA-256).

use emu_core::model::{Component, ComponentId, HostArch, HostOs};
use emu_core::{CoreError, Result};
use url::Url;

/// Every relative `<url>` in the manifest resolves against this.
pub const BASE_URL: &str = "https://dl.google.com/android/repository/";

/// Parse `xml` and resolve the milestone-M1 [`ComponentId::m1_set`] components for `(os, arch)`.
///
/// # Errors
///
/// Returns [`CoreError::Parse`] for malformed XML, [`CoreError::NotFound`] when a package is
/// absent from the manifest, or when no archive matches the given host.
pub fn parse(xml: &[u8], os: HostOs, arch: HostArch) -> Result<Vec<Component>> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| CoreError::parse("repository xml", format!("not valid utf-8: {e}")))?;
    let doc =
        roxmltree::Document::parse(text).map_err(|e| CoreError::parse("repository xml", e))?;

    ComponentId::m1_set()
        .into_iter()
        .map(|id| resolve_component(&doc, id, os, arch))
        .collect()
}

fn resolve_component(
    doc: &roxmltree::Document<'_>,
    id: ComponentId,
    os: HostOs,
    arch: HostArch,
) -> Result<Component> {
    let node = doc
        .descendants()
        .find(|n| n.has_tag_name("remotePackage") && n.attribute("path") == Some(id.repo_path()))
        .ok_or_else(|| CoreError::NotFound {
            what: "SDK component in repository manifest",
            name: id.repo_path().to_string(),
        })?;

    let version = revision_string(&node).ok_or_else(|| {
        CoreError::parse(
            "repository xml",
            format!("`{}` has no <revision><major>", id.repo_path()),
        )
    })?;

    let archive = node
        .descendants()
        .filter(|n| n.has_tag_name("archive"))
        .find(|a| archive_matches(a, os, arch))
        .ok_or_else(|| CoreError::NotFound {
            what: "SDK component archive for this host",
            name: format!("{} on {}/{}", id.repo_path(), os.tag(), arch.tag()),
        })?;

    let complete = archive
        .children()
        .find(|n| n.has_tag_name("complete"))
        .ok_or_else(|| {
            CoreError::parse(
                "repository xml",
                format!("`{}` archive has no <complete>", id.repo_path()),
            )
        })?;

    let size_bytes = text_of(&complete, "size")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| CoreError::parse("repository xml", "missing or non-numeric <size>"))?;

    let sha1 = complete
        .children()
        .find(|n| n.has_tag_name("checksum") && n.attribute("type") == Some("sha1"))
        .and_then(|n| n.text())
        .map(str::to_string)
        .ok_or_else(|| CoreError::parse("repository xml", "missing sha1 <checksum>"))?;

    let url_tail = text_of(&complete, "url")
        .ok_or_else(|| CoreError::parse("repository xml", "missing <url>"))?;
    let base = Url::parse(BASE_URL).expect("BASE_URL is a valid, constant URL");
    let url = base
        .join(url_tail)
        .map_err(|e| CoreError::parse("repository xml", format!("bad <url> `{url_tail}`: {e}")))?;

    Ok(Component {
        id,
        version,
        url,
        size_bytes,
        sha1,
    })
}

/// `true` when `archive`'s `<host-os>` matches `os` and its `<host-arch>` (if present) matches
/// `arch`. A missing `<host-arch>` means the archive applies to every arch on that OS.
fn archive_matches(archive: &roxmltree::Node<'_, '_>, os: HostOs, arch: HostArch) -> bool {
    let Some(archive_os) = text_of(archive, "host-os") else {
        return false;
    };
    if archive_os != os.tag() {
        return false;
    }
    match text_of(archive, "host-arch") {
        Some(archive_arch) => archive_arch == arch.tag(),
        None => true,
    }
}

/// Text content of the first direct child named `tag`, if any.
fn text_of<'a>(node: &roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children().find(|n| n.has_tag_name(tag))?.text()
}

/// Join `<major>[.<minor>[.<micro>]]` with `.`, skipping absent trailing parts.
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

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/repository2-3.xml");

    #[test]
    fn resolves_linux_x64() {
        let got = parse(FIXTURE, HostOs::Linux, HostArch::X64).expect("parse");
        assert_eq!(got.len(), 3);

        let cmdline = &got[0];
        assert_eq!(cmdline.id, ComponentId::CmdlineTools);
        assert_eq!(cmdline.version, "23.0");
        assert_eq!(
            cmdline.url.as_str(),
            "https://dl.google.com/android/repository/commandlinetools-linux-16111833_latest.zip"
        );
        assert_eq!(cmdline.sha1, "e025545c62a8e64c7559119566a569fb1dec5f60");
        assert_eq!(cmdline.size_bytes, 181_052_239);

        let platform_tools = &got[1];
        assert_eq!(platform_tools.version, "37.0.1");
        assert!(platform_tools.url.as_str().ends_with("-linux.zip"));

        let emulator = &got[2];
        assert_eq!(emulator.version, "37.2.7");
        assert!(emulator.url.as_str().contains("emulator-linux_x64"));
    }

    #[test]
    fn resolves_macos_arm64_including_arch_specific_archives() {
        let got = parse(FIXTURE, HostOs::MacOs, HostArch::Arm64).expect("parse");
        assert!(got[0].url.as_str().contains("mac_arm64"));
        // platform-tools has no <host-arch> on macOS — same darwin archive for both arches.
        assert!(got[1].url.as_str().ends_with("-darwin.zip"));
        assert!(got[2].url.as_str().contains("darwin_aarch64"));
    }

    #[test]
    fn resolves_windows_x64() {
        let got = parse(FIXTURE, HostOs::Windows, HostArch::X64).expect("parse");
        assert!(got[0].url.as_str().contains("commandlinetools-win-"));
        assert!(got[1].url.as_str().ends_with("-win.zip"));
        assert!(got[2].url.as_str().contains("emulator-windows_x64"));
    }

    #[test]
    fn missing_archive_for_host_is_not_found_not_a_panic() {
        // The real feed does not publish a windows/aarch64 build of `emulator`.
        let err = parse(FIXTURE, HostOs::Windows, HostArch::Arm64).unwrap_err();
        assert_eq!(err.code(), "not_found");
        assert!(err.to_string().contains("emulator"));
    }

    #[test]
    fn unknown_package_is_not_found() {
        // A manifest with no <remotePackage> elements at all reproduces "package not found"
        // without depending on which package `parse` happens to look for first.
        let empty = b"<sdk:sdk-repository xmlns:sdk=\"http://schemas.android.com/sdk/android/repo/repository2/03\"/>";
        let err = parse(empty, HostOs::Linux, HostArch::X64).unwrap_err();
        assert_eq!(err.code(), "not_found");
        assert!(err.to_string().contains("cmdline-tools;latest"));
    }

    #[test]
    fn malformed_xml_is_a_parse_error_not_a_panic() {
        let err = parse(b"<not-xml", HostOs::Linux, HostArch::X64).unwrap_err();
        assert_eq!(err.code(), "parse_error");
    }
}
