//! `emu-core` — domain logic and orchestration for EmuManager.
//!
//! This crate holds all business logic and defines the **port traits** for every side effect
//! (process spawning, downloads, host probing, the clock, the filesystem). It has **no `tauri`
//! dependency** so it stays testable with `cargo test` alone — see `AGENTS.md` §6 and
//! `docs/architecture.md` §2.
//!
//! Layout:
//! - [`model`] — the domain entities (`docs/context/domain-model.md`). Pure data, `serde`-ready.
//! - [`ports`] — the injected-effect traits and their DTOs.
//! - [`provider`] — the [`provider::Provider`] trait an emulator backend implements.
//! - [`error`] — [`CoreError`] and its stable [`CoreError::code`] contract.
//! - `testing` (cargo feature) — in-memory fakes for every port.
//!
//! Task `0002` adds the types and traits; there is still **no real behavior** here.
//! `specta::Type` derives are added alongside the IPC layer in task `0004`.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod ports;
pub mod provider;
pub mod registry;
pub mod toolchain;

#[cfg(feature = "testing")]
pub mod testing;

pub use error::{CoreError, Result};
