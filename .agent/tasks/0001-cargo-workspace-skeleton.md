---
id: "0001"
title: "Cargo workspace skeleton"
milestone: "M0"
status: "todo"
owner: ""
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

- [ ] `cargo build --workspace` succeeds on the current OS
- [ ] Workspace members: `src-tauri`, `crates/emu-core`, `crates/emu-android`, `crates/emu-host`,
      `crates/emu-helper`
- [ ] `emu-android` and `emu-host` depend on `emu-core`; `src-tauri` depends on all four
- [ ] `emu-core/Cargo.toml` has **no** `tauri` in `[dependencies]` or `[dev-dependencies]`
- [ ] `[workspace.lints]` / `clippy.toml` set up so `cargo clippy --workspace -- -D warnings` is clean
- [ ] `emu-helper` builds as a standalone binary with a `--help` (clap) listing `check`,
      `enable-whpx`, `enable-aehd`, `add-kvm-group` (stubs returning "not implemented")
- [ ] Minimal `tauri.conf.json` with the app id `com.emumanager.app`, an empty capabilities set

## Validate

```
cargo build --workspace && cargo clippy --workspace -- -D warnings
```

## Notes / findings
