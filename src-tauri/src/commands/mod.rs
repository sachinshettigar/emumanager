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
