---
id: "0011"
title: "src-tauri: real ProcessRunner + Downloader + Clock + Fs port impls"
milestone: "M1"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Real, non-fake implementations of the four leaf ports (`ProcessRunner`, `Downloader`, `Clock`,
`Fs`) that `crates/emu-core/src/ports.rs` defines, so M1's toolchain manager (task 0012) has
something real to run against. These are thin OS/network adapters with no domain logic — they
belong in `src-tauri` per `docs/architecture.md` §2 ("constructs the real port impls and injects
them into `emu-core`"), not in `emu-android` (which only *consumes* `ProcessRunner`, per the same
table) or `emu-core` (must stay `tauri`-free).

## Context / links

- Architecture: `docs/architecture.md` §2 (crate boundary table), §3 (port trait signatures)
- Ports being implemented: `crates/emu-core/src/ports.rs`

## Scope — files this task may touch

- `src-tauri/src/ports/mod.rs`, `src-tauri/src/ports/process.rs`, `.../downloader.rs`,
  `.../clock.rs`, `.../fs.rs` (new)
- `src-tauri/Cargo.toml` (add `reqwest` — rustls-tls, no native-tls system dep — and confirm
  `tokio` process/fs features)
- `src-tauri/src/lib.rs` (wire the real impls where `run()` constructs `emu-core` state — no
  command uses them yet, that's task 0012/0013; a `#[allow(dead_code)]`-free construction that at
  least builds and is unit-tested in isolation is enough here)

## Acceptance criteria

- [ ] `NativeProcessRunner`: `run`/`spawn` via `tokio::process::Command`, no shell; `spawn`
      returns a `ChildProcess` that streams merged stdout/stderr line-by-line and `kill()`s the
      real child
- [ ] `NativeDownloader`: streams an HTTP GET (`reqwest`) to a temp file in the same directory as
      `into` (atomic rename on success, per the `Fs::write_atomic` pattern already used
      elsewhere), reports `Progress` via `JobHandle` at a bounded rate (not per-chunk — e.g. only
      when the percentage changes), verifies SHA-256 when `expected_sha256` is `Some`. **Google's
      repo XML gives SHA-1, not SHA-256** (see task 0010) — document in Notes how task 0012
      reconciles that (likely: verify SHA-1 itself before calling `Downloader::fetch` with
      `expected_sha256: None`, or widen the trait; decide and record, don't silently drop
      verification)
- [ ] `SystemClock`: `Clock::now()` via `OffsetDateTime::now_utc()`
- [ ] `NativeFs`: real `tokio::fs` behind the trait, `write_atomic` via temp-file-plus-rename in
      the same directory
- [ ] Unit tests for each against real temp dirs / a local test HTTP server (no real internet
      call in `just validate` — use a crate already common for this, e.g. spin up a tiny
      `tokio::net::TcpListener` responder, or gate a real-network case behind `#[ignore]`)
- [ ] `just check-fast` passes; no new warnings from `clippy -D warnings`

## Validate

```
cargo test -p emumanager --all-features
just check-fast
```

## Notes / findings

(SHA-1 vs SHA-256 reconciliation decision goes here once made — do not decide silently in code.)
