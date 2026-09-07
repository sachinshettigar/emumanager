//! Tauri commands — the typed IPC surface.
//!
//! Every function here is a thin adapter: validate input, call into `emu-core`
//! or a provider, map the result into a serializable DTO or [`IpcError`]. No
//! domain logic lives in this file (`AGENTS.md` §6).

// `#[tauri::command]` deserializes each argument from the IPC payload, so args
// are owned (`String`, not `&str`) even when a body only borrows them.
#![allow(clippy::needless_pass_by_value)]

// `pub(crate)`, not a `pub use` re-export: `#[tauri::command]`/`#[specta::specta]` generate
// hidden sibling items (`__cmd__*`, `__specta__fn__*`) next to each function they annotate, and
// `collect_commands!`/`generate_handler!` look those up at the function's *defining* path — a
// re-export moves the visible name but not those siblings. `lib.rs` refers to
// `commands::toolchain::{list_components, bootstrap_toolchain}` directly instead.
pub(crate) mod device;
pub(crate) mod emulator;
pub(crate) mod host;
pub(crate) mod profile;
pub(crate) mod toolchain;

use serde::{Deserialize, Serialize};

use crate::ipc_error::IpcError;

/// Reply from [`ping`]. Proves the Rust↔TS seam and version wiring are live.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Pong {
    /// Greeting echoing the caller's `name`.
    pub message: String,
    /// The running backend's crate version (`CARGO_PKG_VERSION`).
    pub version: String,
}

/// Static app metadata for the About screen — version, licence, repo, and a one-line-per-feature
/// summary. No I/O.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    /// `CARGO_PKG_VERSION`.
    pub version: String,
    /// `CARGO_PKG_LICENSE` — `"UNLICENSED"` today; a real OSS licence is M7's `LICENSE`-chosen line.
    pub license: String,
    pub repository: String,
    pub description: String,
    /// One short line per headline feature.
    pub features: Vec<String>,
}

/// Return [`AppInfo`]. Everything is a compile-time constant.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Emulator Studio".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        license: option_env!("CARGO_PKG_LICENSE")
            .unwrap_or("UNLICENSED")
            .to_owned(),
        repository: option_env!("CARGO_PKG_REPOSITORY")
            .unwrap_or("https://github.com/sachinshettigar/emumanager")
            .to_owned(),
        description:
            "Create, launch, track and share Android emulators — no Android Studio, no terminal."
                .to_owned(),
        features: vec![
            "Installs the Android command-line SDK (cmdline-tools, platform-tools, emulator, \
             system images) into its own data directory — or reuses an SDK you already have."
                .to_owned(),
            "Each SDK component installs on its own, with live download/progress and logs."
                .to_owned(),
            "Create wizard: pick a device profile and system image, set RAM/storage, review, \
             create — optionally launching straight away."
                .to_owned(),
            "Dashboard tracks every emulator and its live Stopped / Booting / Running state; \
             launch, stop, wipe, delete, rename from there."
                .to_owned(),
            "Per-emulator detail panel with a live log console and an 'open folder' shortcut."
                .to_owned(),
            "Registry reconciles with reality on startup and on demand — adopts AVDs made \
             elsewhere, recovers cleanly after a crash."
                .to_owned(),
            "Portable .emuprofile recipes: export an emulator, import one and it resolves + \
             downloads what's missing and recreates it. Recipes carry no SDK or image bytes."
                .to_owned(),
            "Host-readiness check: virtualization, accelerator (KVM / WHPX / AEHD / HVF), disk \
             and RAM, with a verdict and one-click fixes via an elevated helper."
                .to_owned(),
            "Self-updates from signed release artifacts.".to_owned(),
        ],
    }
}

/// Liveness check. Returns a greeting plus the backend version.
///
/// `name` is rejected when blank so the frontend has a real error path to render
/// against before any milestone-1 command exists.
#[tauri::command]
#[specta::specta]
pub fn ping(name: String) -> Result<Pong, IpcError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(IpcError::from(emu_core::CoreError::invalid(
            "name",
            "must not be blank",
        )));
    }
    Ok(Pong {
        message: format!("pong, {trimmed}"),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::ping;

    #[test]
    fn ping_greets_and_reports_version() {
        let pong = ping("  Ada  ".to_owned()).expect("non-blank name succeeds");
        assert_eq!(pong.message, "pong, Ada");
        assert_eq!(pong.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn ping_rejects_blank_name_with_invalid_code() {
        let err = ping("   ".to_owned()).expect_err("blank name is rejected");
        assert_eq!(err.code, "invalid");
        assert!(
            err.message.contains("name"),
            "message names the field: {}",
            err.message
        );
    }
}
