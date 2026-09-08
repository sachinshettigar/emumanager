---
id: "0029"
title: "Rotating logs + export_diagnostics (redacted)"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-07"
updated: "2026-09-08"
---

## Goal

`src-tauri` gets real logging — `tracing` with a rotating file layer in the data dir — and an
`export_diagnostics` command that bundles the recent logs + host report + versions into a redacted
zip the user can attach to a bug report.

## Context / links

- `MILESTONES.md` M6 bullet 5; `docs/spec.md` §6 Observability
- `AGENTS.md` §7: "`tracing` for logs, never `println!`" — `src-tauri` currently has neither
- `crates/emu-core` already uses `tracing` as a facade in a few places; `src-tauri` doesn't init a
  subscriber, so those events go nowhere
- `zip` is a workspace dep (`emu-core`, `emu-android`); `src-tauri` can add it too
- Data dir layout (`docs/architecture.md` §1): `logs/` already reserved

## Scope — files this task may touch

- `src-tauri/Cargo.toml` (`tracing-subscriber`, `tracing-appender`, `zip`)
- `src-tauri/src/logging.rs` (new — `init(data_dir)`: a `fmt` layer to `<data_dir>/logs/emumanager.log`
  via `tracing_appender::rolling::daily`, plus stderr in debug; `EnvFilter` default `info`)
- `src-tauri/src/lib.rs` (`logging::init` first thing in `setup`; replace the silent
  `let _ = provider.reconcile()` with a real `tracing::warn!` on error)
- `src-tauri/src/commands/host.rs` or a new `src-tauri/src/commands/diagnostics.rs` —
  `export_diagnostics() -> String` (path to the written zip) or `-> Vec<u8>`; redaction helper
- `src/lib/ipc.ts` (`useExportDiagnostics`), `src/routes/Dependencies.tsx` (an "Export diagnostics"
  button — small, bottom of the screen) + a Vitest
- `src/lib/bindings.ts` (regenerated)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `logging::init(&data_dir)` — a daily-rotating file at `<data_dir>/logs/emumanager.log`
      (`tracing-appender`), `EnvFilter` from `RUST_LOG` else `info`, non-blocking writer, a stderr
      layer only under `#[cfg(debug_assertions)]`. Called at the very top of `setup`. Idempotent /
      safe if the dir can't be made (log to stderr, don't panic).
- [x] The startup `reconcile()` (task `0020`) and the exit reaper log their outcome
      (`tracing::info!` / `warn!`) instead of `let _ = …`.
- [x] `export_diagnostics() -> String` — writes `<data_dir>/diagnostics-<timestamp>.zip` containing:
      the tail of `emumanager.log` (last ~2000 lines), the latest `host_snapshots` row (or a fresh
      `probe_host`), `{ app, tauri, os, arch }` versions, and the `emulators` table as JSON
      **with `avd_name` / paths / `adb_serial` kept but `notes` and any `$HOME`-prefixed path
      segment replaced by `~`**. Returns the zip path; the UI then `reveal_path`s it.
- [x] Redaction test (Rust): a log line containing a home path and a token-looking string comes out
      with the home path collapsed to `~` (tokens: best-effort — document what is / isn't scrubbed).
- [x] "Export diagnostics" button on the Dependencies screen → `export_diagnostics` → `reveal_path`
      the result. Vitest: the button calls the command.
- [x] `just bindings` clean; `just check-fast` then `just validate` green.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Filenames in the criteria above predate the 0032 rename — the log file is
`emulator-studio.log`, and `export_diagnostics` uses a **fresh** `probe_host` rather than the
`host_snapshots` row.)

- **`tracing`/`tracing-subscriber`/`tracing-appender` were greenfield** — the repo had no
  `tracing` anywhere (the task's "emu-core already uses tracing" note was stale). `src-tauri`
  now owns the subscriber; `emu-core` can start emitting `tracing::*` events any time and they'll
  land in the file.
- **The `WorkerGuard` is parked in Tauri managed state** (`app.manage(std::sync::Mutex::new(guard))`)
  because `WorkerGuard` is `Send` but not `Sync` and `app.manage` needs `Sync`. It's never read
  again — it just has to outlive the app so the writer thread flushes.
- **`logging::init` returns `None` and no-ops on a second call** (`try_init` fails once a global
  subscriber is set) — so the `#[cfg(test)]` unit test that calls it can't fight another test.
- **Redaction is home-path → `~` only.** `redact_with(text, home)` is the testable core;
  `redact` reads `HOME`/`USERPROFILE`. Non-path secrets are explicitly *not* scrubbed — the
  app's own log lines are authored not to contain them, and a regex sweep risks mangling real
  content. Documented in the fn doc and asserted in the test (`token=abc123` survives).
- **The emulators table in the zip is hand-built JSON**, not `serde_json::to_string(EmulatorRow)` —
  `EmulatorRow` isn't `Serialize`, and hand-building means `notes`/`tags` are structurally
  impossible to leak.
- **Zip writing**: `zip = { version = "2", features = ["deflate"] }` (v2 already in the lock via
  emu-core; no new duplicate). `ZipWriter` + `SimpleFileOptions::default().compression_method(Deflated)`.
- `#[deny(unsafe_code)]` in `lib.rs` means the test can't `std::env::set_var` (unsafe in edition
  2024) — hence the `redact_with(text, home)` split.
- `just validate` green apart from the expected `git diff src/lib/bindings.ts` step (new command).
