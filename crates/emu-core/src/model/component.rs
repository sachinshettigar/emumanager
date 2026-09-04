//! SDK components the toolchain manager can fetch — the pieces needed to get from an empty data
//! dir to a working `sdkmanager` (milestone M1). System images have their own coordinate type
//! ([`crate::model::image::ImageCoord`]); this module covers the three non-image components.
//!
//! Tag strings on [`ComponentId`], [`HostOs`], and [`HostArch`] are load-bearing: they are the
//! exact `path`, `<host-os>`, and `<host-arch>` values Google's repository manifest uses. Cited
//! against a real captured fixture in `crates/emu-android/tests/fixtures/repository2-3.xml`
//! (source: <https://dl.google.com/android/repository/repository2-3.xml>).

use serde::{Deserialize, Serialize};
use url::Url;

/// One of the SDK components the app manages outside of system images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ComponentId {
    /// `cmdline-tools;latest` — ships `sdkmanager`/`avdmanager`.
    CmdlineTools,
    /// `platform-tools` — ships `adb`.
    PlatformTools,
    /// `emulator` — ships the `emulator` binary.
    Emulator,
}

impl ComponentId {
    /// The exact `sdkmanager` package path (the XML `<remotePackage path="...">` attribute).
    #[must_use]
    pub const fn repo_path(self) -> &'static str {
        match self {
            ComponentId::CmdlineTools => "cmdline-tools;latest",
            ComponentId::PlatformTools => "platform-tools",
            ComponentId::Emulator => "emulator",
        }
    }

    /// All components M1 bootstraps, in install order (`cmdline-tools` must land first).
    #[must_use]
    pub const fn m1_set() -> [ComponentId; 3] {
        [
            ComponentId::CmdlineTools,
            ComponentId::PlatformTools,
            ComponentId::Emulator,
        ]
    }
}

/// Host operating system, as Google's repository manifest spells it in `<host-os>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostOs {
    /// `linux`.
    Linux,
    /// `macosx`.
    MacOs,
    /// `windows`.
    Windows,
}

impl HostOs {
    /// The exact `<host-os>` tag text.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            HostOs::Linux => "linux",
            HostOs::MacOs => "macosx",
            HostOs::Windows => "windows",
        }
    }

    /// The running process's host OS, as Rust's `std::env::consts::OS` reports it.
    ///
    /// Returns `None` on an OS the SDK doesn't publish archives for (there is no such target
    /// today, but this keeps the mapping honest instead of guessing).
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::OS {
            "linux" => Some(HostOs::Linux),
            "macos" => Some(HostOs::MacOs),
            "windows" => Some(HostOs::Windows),
            _ => None,
        }
    }
}

/// Host CPU architecture, as Google's repository manifest spells it in `<host-arch>`.
///
/// Archives with no `<host-arch>` element apply to every architecture on that OS (see
/// `platform-tools` in the fixture) — that case is represented at the parser call site, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostArch {
    /// `x64`.
    X64,
    /// `aarch64`.
    Arm64,
}

impl HostArch {
    /// The exact `<host-arch>` tag text.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            HostArch::X64 => "x64",
            HostArch::Arm64 => "aarch64",
        }
    }

    /// The running process's host architecture, as Rust's `std::env::consts::ARCH` reports it.
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::ARCH {
            "x86_64" => Some(HostArch::X64),
            "aarch64" => Some(HostArch::Arm64),
            _ => None,
        }
    }
}

/// One resolved, host-matched archive for a [`ComponentId`], ready to hand to a `Downloader`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// Which component this is.
    pub id: ComponentId,
    /// Package revision, dot-joined (`major[.minor[.micro]]`), e.g. `"37.0.1"`.
    pub version: String,
    /// Fully resolved download URL (relative `<url>` values are joined against the repository
    /// base URL — see `emu_android::catalog`).
    pub url: Url,
    /// Archive size in bytes.
    pub size_bytes: u64,
    /// Lower-case hex SHA-1 of the archive (Google's manifest does not publish SHA-256).
    pub sha1: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_paths_match_sdkmanager_package_ids() {
        assert_eq!(
            ComponentId::CmdlineTools.repo_path(),
            "cmdline-tools;latest"
        );
        assert_eq!(ComponentId::PlatformTools.repo_path(), "platform-tools");
        assert_eq!(ComponentId::Emulator.repo_path(), "emulator");
    }

    #[test]
    fn m1_set_is_cmdline_tools_first() {
        assert_eq!(ComponentId::m1_set()[0], ComponentId::CmdlineTools);
    }

    #[test]
    fn host_tags_match_manifest_strings() {
        assert_eq!(HostOs::Linux.tag(), "linux");
        assert_eq!(HostOs::MacOs.tag(), "macosx");
        assert_eq!(HostOs::Windows.tag(), "windows");
        assert_eq!(HostArch::X64.tag(), "x64");
        assert_eq!(HostArch::Arm64.tag(), "aarch64");
    }
}
