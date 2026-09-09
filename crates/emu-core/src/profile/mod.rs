//! Profile engine — resolve an [`EmuProfile`](crate::model::profile::EmuProfile) against local
//! install state into a runnable [`Plan`](crate::model::plan::Plan). The reverse direction
//! (export a tracked emulator back to a recipe) lives on `EmuProfile` itself
//! (`EmuProfile::from_emulator` / `from_create_spec`).
//!
//! Pure and effect-free: the caller supplies the two facts `emu-core` can't observe — whether the
//! target system image is installed, and its download size — so this stays `cargo test`-able with
//! no ports.

mod resolve;

pub use resolve::{resolve, sanitize_avd_name, unique_avd_name, unique_display_name};
