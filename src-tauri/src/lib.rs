//! EmuManager Tauri shell.
//!
//! This crate is **glue only** — Tauri commands, event emitters, and dependency wiring. All
//! domain logic lives in `emu-core` and the provider crates (`AGENTS.md` §6,
//! `docs/architecture.md` §2). Commands and the typed `tauri-specta` bindings land in task `0004`.

#![deny(unsafe_code)]

/// Build and run the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the EmuManager application");
}
