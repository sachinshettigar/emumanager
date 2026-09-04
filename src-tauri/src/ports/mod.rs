//! Real, OS/network-facing implementations of `emu-core`'s ports (`emu_core::ports`).
//!
//! These are thin adapters with no domain logic — `docs/architecture.md` §2 puts them here
//! (`src-tauri` "constructs the real port impls and injects them into `emu-core`"), not in
//! `emu-android` (which only *consumes* an injected `ProcessRunner`) or `emu-core` (which must
//! stay `tauri`-free).
//!
//! No command wires these in yet — that lands with the toolchain manager (task 0012) and the
//! Dependencies screen (task 0013). Each impl is unit-tested in isolation (real temp dirs, a
//! one-shot local HTTP server for the downloader) so this module is independently correct before
//! anything calls it. `#[allow(dead_code)]`: every trait method here (`fetch`/`run`/`spawn`/
//! `now`/...) is only exercised by those tests until then — constructing-but-never-calling the
//! structs elsewhere just to silence the lint would add real startup cost (an HTTP client, etc.)
//! for no behavior, so the allow is the honest option. Remove it the moment task 0012 calls any
//! of these for real. `unused_imports` is allowed for the same reason on the re-exports below —
//! nothing imports `crate::ports::*` yet either.
#![allow(dead_code, unused_imports)]

mod clock;
mod downloader;
mod fs;
mod process;

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
