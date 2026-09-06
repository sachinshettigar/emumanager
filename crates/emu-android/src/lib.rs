//! `emu-android` — implements the `Provider` port by driving the Android command-line tools
//! (`sdkmanager`, `avdmanager`, `emulator`, `adb`) via the injected `ProcessRunner`.
//!
//! Output parsing lives here and is tested against captured fixtures in `tests/fixtures/`
//! (see `docs/playbooks/write-tests.md`). Implementations land from task `0002` onward.

#![forbid(unsafe_code)]

pub mod avd_list;
pub mod catalog;
pub mod devices;
pub mod provider;
pub mod sysimg;

pub use emu_core::{CoreError, Result};
