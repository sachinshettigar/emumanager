# 2026-09-05 — session 5 (Claude Code)

## Worked on

- Task(s): `0011` (done)
- Milestone: `M1`

## Changed

- `src-tauri/src/ports/{mod,clock,downloader,fs,process}.rs` (new) — real
  `NativeProcessRunner`/`NativeDownloader`/`SystemClock`/`NativeFs`.
- `src-tauri/Cargo.toml` — added `tokio`, `reqwest` (`rustls`, `stream`), `sha2`, `url`,
  `futures-util`, `time`, `async-trait`; `tempfile` dev-dep.
- `deny.toml` — allow-listed `CDLA-Permissive-2.0` (real `cargo-deny` failure this task's own
  `just validate` run surfaced — see task file Notes).
- `.agent/tasks/0011-native-ports.md` → `done`; `.agent/tasks/0012-toolchain-bootstrap.md` —
  amended with a user-requested "recognize an existing system SDK, don't re-download" acceptance
  criterion.
- `.agent/state.json`, `PROGRESS.md` updated.

## State now

- `just validate`: **pass**
- `just progress`: pass
- Tasks moved: `0011` doing→done
- `lastValidatedCommit` in state.json: set to this session's commit after pushing

## Next action

Task `0012` — toolchain bootstrap + `InstalledState` in `emu-core`. Three things must be decided
there, not assumed:

1. SHA-1 (Google's manifest, task 0010) vs. SHA-256 (`Downloader::fetch`, task 0011) —
   reconcile deliberately.
2. Bundled JRE (per `docs/spec.md` §5.1) vs. requiring a system JDK (what modern `cmdline-tools`
   actually needs) — task file recommends (a) require a system JDK with a clear error for v1.
3. **New this session, from the user directly:** `InstalledState` must check for an existing
   system Android SDK (`ANDROID_HOME`/`ANDROID_SDK_ROOT`, or the OS-conventional Android Studio
   path — `~/Library/Android/sdk` macOS, `~/Android/Sdk` Linux, `%LOCALAPPDATA%\Android\Sdk`
   Windows) *before* concluding a component needs downloading. `bootstrap()` must skip anything
   already satisfied by a system install — no copy, no re-download. See task 0012's updated Goal
   and acceptance criteria for the full requirement.

## Gotchas / notes for the next agent

- `src-tauri/src/ports/mod.rs` carries a deliberate `#![allow(dead_code, unused_imports)]` —
  remove it the moment task 0012 actually constructs and calls one of these ports for real (a
  command, or `bootstrap()`'s own wiring). Don't just leave it there out of habit.
- `NativeDownloader::fetch` verifies SHA-256 only; it does not know about SHA-1 at all. Whatever
  0012 decides, it happens at the *call site* (0012's code), not by changing this port's trait
  signature casually — check `crates/emu-core/src/ports.rs`'s `Downloader` trait first if a
  signature change starts to seem necessary, since other things may already assume its shape.
- `NativeProcessRunner`'s `spawn()` merges stdout+stderr for `next_line()`; if task 0012 needs to
  tell them apart for some reason (unlikely — license-prompt handling reads stdin, not stdout),
  that's a real limitation to revisit, not a bug to silently work around.
- CI is still blocked on GitHub billing (Settings → Billing & plans), unrelated to any of this —
  see the session-4 journal entry for the full story. Nothing to do there until a human fixes it.
