//! What's actually on disk right now — scanned fresh every time, never cached, matching the
//! `SystemImage::installed` pattern (`crate::model::image`).
//!
//! Checks two kinds of root, per the M1 "reuse what's already there" requirement:
//! - the app's own managed `<data_dir>/sdk/` directory (where [`super::bootstrap::bootstrap`]
//!   installs things), and
//! - any *existing system Android SDK* — `ANDROID_HOME`/`ANDROID_SDK_ROOT`, or the OS-conventional
//!   Android Studio install path — so a machine that already has Android Studio never gets a
//!   redundant multi-gigabyte re-download.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::model::component::{ComponentId, HostOs};
use crate::ports::Fs;

/// Where a component's install was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkSource {
    /// Inside the app's own managed `<data_dir>/sdk/` directory.
    AppManaged,
    /// An existing system-wide Android SDK, rooted at this path (env var or Android Studio
    /// default location — see [`system_roots`]).
    System(PathBuf),
}

/// One component, found on disk, and where.
///
/// Later `create`/`launch` code needs the root, not just a yes/no, to know which
/// `sdkmanager`/`adb`/`emulator` binary to invoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentLocation {
    /// Which component this is.
    pub id: ComponentId,
    /// The SDK root it was found under (an app-managed `sdk/` dir, or a system root).
    pub sdk_root: PathBuf,
    /// App-managed vs. an existing system install.
    pub source: SdkSource,
}

/// A snapshot of what's installed. Build with [`scan`]; never construct or cache stale — re-scan
/// whenever the answer matters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledState {
    found: Vec<ComponentLocation>,
}

impl InstalledState {
    /// Build a state directly from known locations (e.g. re-hydrating one saved earlier). Real
    /// callers get one from [`scan`] instead.
    #[must_use]
    pub fn from_found(found: Vec<ComponentLocation>) -> Self {
        Self { found }
    }

    /// Where `id` was found, if at all.
    #[must_use]
    pub fn location_of(&self, id: ComponentId) -> Option<&ComponentLocation> {
        self.found.iter().find(|c| c.id == id)
    }

    /// `true` if `id` is installed anywhere (app-managed or system).
    #[must_use]
    pub fn is_installed(&self, id: ComponentId) -> bool {
        self.location_of(id).is_some()
    }

    /// Of `wanted`, the ones not found anywhere — what [`super::bootstrap::bootstrap`] must fetch.
    #[must_use]
    pub fn missing(&self, wanted: &[ComponentId]) -> Vec<ComponentId> {
        wanted
            .iter()
            .copied()
            .filter(|id| !self.is_installed(*id))
            .collect()
    }
}

/// The marker file (relative to an SDK root) that proves a component is genuinely installed
/// there — the one file each component's own package guarantees it lays down, not an invented
/// convention:
///
/// - `cmdline-tools;latest` → `cmdline-tools/latest/bin/sdkmanager(.bat)` — the entry point
///   [`super::bootstrap::bootstrap`] itself goes on to invoke.
/// - `platform-tools` → `platform-tools/adb(.exe)`.
/// - `emulator` → `emulator/emulator(.exe)`.
///
/// `.bat`/`.exe` on Windows, no suffix elsewhere — cited against the real archive layout captured
/// while building task 0012 (see the task file's Notes).
/// Public wrapper around [`marker_file`]: the real binary to invoke for `id`, given the SDK root
/// it was found under (from a [`ComponentLocation`]). `create`/`launch` code (M2+) uses this to
/// know which `sdkmanager`/`adb`/`emulator` to run — the whole reason [`ComponentLocation`]
/// carries a root and not just a bool.
#[must_use]
pub fn binary_path(sdk_root: &Path, id: ComponentId, os: HostOs) -> PathBuf {
    sdk_root.join(marker_file(id, os))
}

pub(super) fn marker_file(id: ComponentId, os: HostOs) -> PathBuf {
    let windows = matches!(os, HostOs::Windows);
    match id {
        ComponentId::CmdlineTools => {
            let bin = if windows {
                "sdkmanager.bat"
            } else {
                "sdkmanager"
            };
            Path::new("cmdline-tools/latest/bin").join(bin)
        }
        ComponentId::PlatformTools => {
            let bin = if windows { "adb.exe" } else { "adb" };
            Path::new("platform-tools").join(bin)
        }
        ComponentId::Emulator => {
            let bin = if windows { "emulator.exe" } else { "emulator" };
            Path::new("emulator").join(bin)
        }
    }
}

/// Candidate *system* SDK roots to check, in priority order — never includes the app-managed dir
/// (the caller checks that separately, first).
///
/// 1. `ANDROID_SDK_ROOT` — the name `sdkmanager`/Android Studio itself prefers today.
/// 2. `ANDROID_HOME` — the older name, still widely set and still honored.
/// 3. The OS-conventional Android Studio SDK install path, when a home/profile directory is
///    known: `~/Library/Android/sdk` (macOS), `~/Android/Sdk` (Linux), `%LOCALAPPDATA%\Android\Sdk`
///    (Windows) — Android Studio's own documented default locations.
///
/// `env` is a plain injected lookup (not a new port trait — this is the one place M1 needs it,
/// and a closure keeps it unit-testable without expanding `ports.rs`).
fn system_roots(os: HostOs, env: &impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        if let Some(v) = env(var) {
            if !v.trim().is_empty() {
                roots.push(PathBuf::from(v));
            }
        }
    }
    let studio_default = match os {
        HostOs::MacOs => env("HOME").map(|h| Path::new(&h).join("Library/Android/sdk")),
        HostOs::Linux => env("HOME").map(|h| Path::new(&h).join("Android/Sdk")),
        HostOs::Windows => env("LOCALAPPDATA").map(|l| Path::new(&l).join(r"Android\Sdk")),
    };
    if let Some(dir) = studio_default {
        roots.push(dir);
    }
    roots
}

/// Scan the filesystem for every M1 component, app-managed dir first, then system roots.
///
/// `os` is passed by the caller (real code passes [`HostOs::current`]) rather than resolved
/// internally, so this stays exercisable for every OS branch in tests regardless of the machine
/// actually running them — the same pattern `emu_android::catalog::parse` uses.
pub async fn scan(
    fs: &dyn Fs,
    app_sdk_dir: &Path,
    os: HostOs,
    env: impl Fn(&str) -> Option<String>,
) -> Result<InstalledState> {
    let mut roots: Vec<(PathBuf, SdkSource)> =
        vec![(app_sdk_dir.to_path_buf(), SdkSource::AppManaged)];
    for root in system_roots(os, &env) {
        roots.push((root.clone(), SdkSource::System(root)));
    }

    let mut found = Vec::new();
    for id in ComponentId::m1_set() {
        for (root, source) in &roots {
            if fs.exists(&root.join(marker_file(id, os))).await? {
                found.push(ComponentLocation {
                    id,
                    sdk_root: root.clone(),
                    source: source.clone(),
                });
                break;
            }
        }
    }
    Ok(InstalledState { found })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InMemoryFs;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[tokio::test]
    async fn nothing_installed_anywhere_is_all_missing() {
        let fs = InMemoryFs::new();
        let state = scan(&fs, Path::new("/data/sdk"), HostOs::Linux, no_env)
            .await
            .expect("scan");
        assert_eq!(state.missing(&ComponentId::m1_set()).len(), 3);
        assert!(!state.is_installed(ComponentId::PlatformTools));
    }

    #[tokio::test]
    async fn app_managed_marker_files_are_found() {
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/data/sdk/cmdline-tools/latest/bin/sdkmanager"),
            b"#!/bin/sh",
        )
        .await
        .unwrap();
        fs.write_atomic(Path::new("/data/sdk/platform-tools/adb"), b"bin")
            .await
            .unwrap();

        let state = scan(&fs, Path::new("/data/sdk"), HostOs::Linux, no_env)
            .await
            .unwrap();
        assert!(state.is_installed(ComponentId::CmdlineTools));
        assert!(state.is_installed(ComponentId::PlatformTools));
        assert!(!state.is_installed(ComponentId::Emulator));
        assert_eq!(
            state.missing(&ComponentId::m1_set()),
            [ComponentId::Emulator]
        );
        assert_eq!(
            state
                .location_of(ComponentId::PlatformTools)
                .unwrap()
                .source,
            SdkSource::AppManaged
        );
    }

    #[tokio::test]
    async fn env_var_system_root_is_detected_and_skips_download() {
        let fs = InMemoryFs::new();
        fs.write_atomic(Path::new("/opt/android-sdk/platform-tools/adb"), b"bin")
            .await
            .unwrap();

        let env = |k: &str| -> Option<String> {
            (k == "ANDROID_SDK_ROOT").then(|| "/opt/android-sdk".to_string())
        };
        let state = scan(&fs, Path::new("/data/sdk"), HostOs::Linux, env)
            .await
            .unwrap();
        let loc = state.location_of(ComponentId::PlatformTools).unwrap();
        assert_eq!(loc.sdk_root, Path::new("/opt/android-sdk"));
        assert_eq!(
            loc.source,
            SdkSource::System(PathBuf::from("/opt/android-sdk"))
        );
    }

    #[tokio::test]
    async fn android_studio_default_path_is_checked_per_os() {
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/Users/dev/Library/Android/sdk/emulator/emulator"),
            b"bin",
        )
        .await
        .unwrap();
        let env = |k: &str| -> Option<String> { (k == "HOME").then(|| "/Users/dev".to_string()) };
        let state = scan(&fs, Path::new("/data/sdk"), HostOs::MacOs, env)
            .await
            .unwrap();
        assert!(state.is_installed(ComponentId::Emulator));

        // The same HOME on Linux checks the Linux-conventional path instead, and does not match
        // the macOS one — the OS distinction is load-bearing, not decorative.
        let state_linux = scan(&fs, Path::new("/data/sdk"), HostOs::Linux, env)
            .await
            .unwrap();
        assert!(!state_linux.is_installed(ComponentId::Emulator));
    }

    #[tokio::test]
    async fn app_managed_wins_over_system_when_both_have_it() {
        let fs = InMemoryFs::new();
        fs.write_atomic(Path::new("/data/sdk/platform-tools/adb"), b"bin")
            .await
            .unwrap();
        fs.write_atomic(Path::new("/opt/sdk/platform-tools/adb"), b"bin")
            .await
            .unwrap();
        let env =
            |k: &str| -> Option<String> { (k == "ANDROID_HOME").then(|| "/opt/sdk".to_string()) };
        let state = scan(&fs, Path::new("/data/sdk"), HostOs::Linux, env)
            .await
            .unwrap();
        assert_eq!(
            state
                .location_of(ComponentId::PlatformTools)
                .unwrap()
                .source,
            SdkSource::AppManaged
        );
    }

    #[test]
    fn marker_files_use_platform_suffix() {
        assert_eq!(
            marker_file(ComponentId::PlatformTools, HostOs::Windows),
            Path::new("platform-tools/adb.exe")
        );
        assert_eq!(
            marker_file(ComponentId::PlatformTools, HostOs::Linux),
            Path::new("platform-tools/adb")
        );
        assert_eq!(
            marker_file(ComponentId::CmdlineTools, HostOs::Windows),
            Path::new("cmdline-tools/latest/bin/sdkmanager.bat")
        );
    }
}
