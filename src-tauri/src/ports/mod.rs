//! Real, OS/network-facing implementations of `emu-core`'s ports (`emu_core::ports`).
//!
//! These are thin adapters with no domain logic — `docs/architecture.md` §2 puts them here
//! (`src-tauri` "constructs the real port impls and injects them into `emu-core`"), not in
//! `emu-android` (which only *consumes* an injected `ProcessRunner`) or `emu-core` (which must
//! stay `tauri`-free).
//!
//! First wired in for real by `crate::commands::toolchain` (task 0013) — `NativeFs` for
//! `InstalledState::scan`, and all three of `NativeFs`/`NativeDownloader`/`NativeProcessRunner`
//! for `bootstrap_toolchain`. `SystemClock` has no caller yet (nothing in M1 needs the wall
//! clock through this seam) and stays behind a narrow, still-honest `#[allow(dead_code)]` on its
//! own module rather than a blanket one here.

mod clock;
mod downloader;
mod fs;
mod process;

// No caller yet — nothing in M1 needs the wall clock through this seam (see the module doc).
#[allow(unused_imports)]
pub use clock::SystemClock;
pub use downloader::NativeDownloader;
pub use fs::NativeFs;
pub use process::NativeProcessRunner;

use std::path::{Path, PathBuf};

/// A sibling temp path for atomic write-then-rename: `<name>.<pid>.tmp` next to `path`.
///
/// Including the process id keeps two concurrent runs (e.g. two test threads) from colliding on
/// the same temp name.
pub(crate) fn tmp_sibling(path: &Path) -> PathBuf {
    let file_name = path.file_name().map_or_else(
        || "download".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()))
}

/// Map a `std::io::Error` at `path` into a [`emu_core::CoreError::Fs`].
pub(crate) fn fs_err(path: &Path, e: &std::io::Error) -> emu_core::CoreError {
    emu_core::CoreError::Fs {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}
