---
id: "0001"
title: "Cargo workspace skeleton"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-04"
---

## Goal

A Cargo workspace that compiles, with the crate boundaries from `docs/architecture.md` §2 in
place (empty but real).

## Scope — files this task may touch

- `Cargo.toml` (workspace root)
- `rustfmt.toml`, `clippy.toml`
- `src-tauri/` (`Cargo.toml`, `src/main.rs`, `tauri.conf.json`, `build.rs`, `capabilities/`)
- `crates/emu-core/` (`Cargo.toml`, `src/lib.rs`)
- `crates/emu-android/` (`Cargo.toml`, `src/lib.rs`)
- `crates/emu-host/` (`Cargo.toml`, `src/lib.rs`)
- `crates/emu-helper/` (`Cargo.toml`, `src/main.rs`)

## Acceptance criteria

- [x] `cargo build --workspace` succeeds on the current OS (aarch64-apple-darwin)
- [x] Workspace members: `src-tauri`, `crates/emu-core`, `crates/emu-android`, `crates/emu-host`,
      `crates/emu-helper`
- [x] `emu-android` and `emu-host` depend on `emu-core`; `src-tauri` depends on all four
- [x] `emu-core/Cargo.toml` has **no** `tauri` in `[dependencies]` or `[dev-dependencies]`
      (`scripts/emu-core-no-tauri.sh` passes)
- [x] `[workspace.lints]` (clippy pedantic + allows) / `clippy.toml` set up;
      `cargo clippy --workspace --all-targets -- -D warnings` is clean; `cargo fmt --check` clean
- [x] `emu-helper` builds as a standalone binary; `--help` lists `check`, `enable-whpx`,
      `enable-aehd`, `add-kvm-group`; each prints `{"status":"not_implemented", ...}` JSON
- [x] Minimal `tauri.conf.json` + empty capability set (`core:default` only)
- [x] `cargo test -p emu-core` passes (1 test: error code stability)

## Validate

```
cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo fmt --all --check && cargo test -p emu-core && bash scripts/emu-core-no-tauri.sh
```

## Notes / findings

- **App identifier is `com.emumanager.desktop`, not `com.emumanager.app`** — Tauri v2 rejects a
  bundle identifier ending in `.app`. Update ADR/docs if a different id is wanted.
- `generate_context!` requires window icons even with `bundle.active: false`. Added
  `scripts/gen-placeholder-icons.mjs` (zero-dep PNG writer) producing
  `src-tauri/icons/{32x32,128x128,128x128@2x,icon}.png`. Real icons = M6. `.icns`/`.ico` not
  needed until bundling.
- `frontendDist` points at `../dist`; committed a placeholder `dist/index.html` (gitignore now
  `dist/*` + `!dist/index.html`). Task 0003 replaces it with the Vite build.
- Crate/bin names for later tasks: bin = `emumanager`, lib = `emumanager_lib`. The `justfile`
  `bindings` recipe still says `-p app` — fix in task 0004/0006.
- Versions in play: rustc 1.98.1, tauri 2.11.5, tauri-build 2.6.3, clap 4.6, thiserror 2.
- Added `rust-toolchain.toml` (pins stable + rustfmt + clippy) for reproducibility across
  harnesses — not in the original scope list but load-bearing for that goal.
