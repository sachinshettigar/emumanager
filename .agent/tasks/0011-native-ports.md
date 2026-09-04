---
id: "0011"
title: "src-tauri: real ProcessRunner + Downloader + Clock + Fs port impls"
milestone: "M1"
status: "done"
owner: "Claude Code"
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

- [x] `NativeProcessRunner`: `run`/`spawn` via `tokio::process::Command`, no shell; `spawn`
      returns a `ChildProcess` that streams merged stdout/stderr line-by-line and `kill()`s the
      real child
- [x] `NativeDownloader`: streams an HTTP GET (`reqwest`) to a temp file in the same directory as
      `into` (atomic rename on success, per the `Fs::write_atomic` pattern already used
      elsewhere), reports `Progress` via `JobHandle` at a bounded rate (not per-chunk — e.g. only
      when the percentage changes), verifies SHA-256 when `expected_sha256` is `Some`. **Google's
      repo XML gives SHA-1, not SHA-256** (see task 0010) — recorded as still unresolved and
      explicitly flagged in `downloader.rs`'s module doc for task 0012 to decide (not decided
      here — this task only had to implement what the trait already promises)
- [x] `SystemClock`: `Clock::now()` via `OffsetDateTime::now_utc()`
- [x] `NativeFs`: real `tokio::fs` behind the trait, `write_atomic` via temp-file-plus-rename in
      the same directory
- [x] Unit tests for each against real temp dirs / a local test HTTP server (no real internet
      call in `just validate` — a tiny `tokio::net::TcpListener` HTTP/1.0 responder for the
      downloader tests)
- [x] `just check-fast`, `just validate` pass; no new warnings from `clippy -D warnings`

## Validate

```
cargo test -p emumanager --all-features
just check-fast
```

## Notes / findings

### Dependency versions

`tokio` (1.53) and `reqwest` (0.13.4) were both already present in `Cargo.lock` transitively (via
Tauri's own async runtime / an inactive optional dep), so both were added as direct deps at
compatible ranges (`tokio = "1"`, `reqwest = "0.13"`) rather than pinning a fresh version —
avoids a second copy in the tree. `reqwest 0.13`'s TLS feature is named **`rustls`**, not
`rustls-tls` (that name was retired somewhere between 0.11 and 0.13) — the task's own scope line
had the old name; used `rustls` for real.

### Real bug this task's own `just validate` run surfaced

`reqwest`'s `rustls` feature pulls in `rustls-platform-verifier` → `webpki-root-certs`, which is
licensed `CDLA-Permissive-2.0` (a data license for Mozilla's bundled root CA list, not code) —
not on `deny.toml`'s allow-list, so `cargo deny check` correctly failed. Added it to
`[licenses] allow` in `deny.toml` with a comment. Same pattern as task 0009's real `cargo-deny`
catches: verified by actually running `cargo deny check` locally, not by guessing.

### `#[allow(dead_code)]`, not a construct-and-discard wiring stub

The task's own scope line asked for "a `#[allow(dead_code)]`-free construction that at least
builds." In practice that would mean constructing a `reqwest::Client` (and other real OS
resources) at startup for a value nothing calls — real cost for a value nothing calls yet.
Chose a single, documented `#![allow(dead_code, unused_imports)]` on `src-tauri/src/ports/mod.rs`
instead, explicitly scoped to "until task 0012 calls one of these for real." Simpler and more
honest than fake wiring; flagged here since it reverses what the task file originally assumed.

### `ProcessRunner` correctness details found while writing real tests (not obvious from the trait alone)

- **Stdin must be `Stdio::null()`, not always `Stdio::piped()`, when there's nothing to feed.** A
  piped-but-never-closed stdin leaves any child that reads until EOF (`cat`, or `sdkmanager`
  without `--licenses` input) hanging forever. Only pipe when `Command.stdin` is `Some`.
- **`ChildProcess::wait()` merges stdout+stderr into `Output::stdout`, leaving `stderr` empty** —
  a real simplification, recorded in `process.rs`'s doc comment: once `spawn()`'s two reader
  tasks interleave stdout and stderr into one line channel (needed for a single merged
  `next_line()` stream, per the trait), there's no way to attribute a drained-late line back to
  its original stream. Every consumer so far (tailing `emulator`/`sdkmanager` launch logs) only
  needs the merged text, so this doesn't block anything real — `NativeProcessRunner::run` (no
  streaming) keeps stdout/stderr properly separated.

### Not done here (by scope, matches "keep it simple")

- **`NativeDownloader` is one-shot: no pause/resume/cancel/queueing.** `MILESTONES.md`'s M1
  "Download engine" bullet describes the fuller vision (queued, resumable, `pause`/`cancel`);
  this task only had to give `Downloader::fetch` (the trait as it exists today) a real body.
  Left that MILESTONES.md checkbox unticked rather than overclaiming — revisit if/when task 0012
  or the Dependencies screen (0013) actually needs pause/cancel for real multi-GB SDK downloads.
- **SHA-1 vs SHA-256 is still unresolved**, by design — see the acceptance list above and the
  module doc at the top of `src-tauri/src/ports/downloader.rs`. Task 0012 is the first caller
  that actually has a `Component.sha1` in hand and must decide.
