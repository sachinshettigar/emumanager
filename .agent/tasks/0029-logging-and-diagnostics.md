---
id: "0029"
title: "Rotating logs + export_diagnostics (redacted)"
milestone: "M6"
status: "todo"
owner: ""
created: "2026-09-07"
updated: "2026-09-07"
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

- [ ] `logging::init(&data_dir)` — a daily-rotating file at `<data_dir>/logs/emumanager.log`
      (`tracing-appender`), `EnvFilter` from `RUST_LOG` else `info`, non-blocking writer, a stderr
      layer only under `#[cfg(debug_assertions)]`. Called at the very top of `setup`. Idempotent /
      safe if the dir can't be made (log to stderr, don't panic).
- [ ] The startup `reconcile()` (task `0020`) and the exit reaper log their outcome
      (`tracing::info!` / `warn!`) instead of `let _ = …`.
- [ ] `export_diagnostics() -> String` — writes `<data_dir>/diagnostics-<timestamp>.zip` containing:
      the tail of `emumanager.log` (last ~2000 lines), the latest `host_snapshots` row (or a fresh
      `probe_host`), `{ app, tauri, os, arch }` versions, and the `emulators` table as JSON
      **with `avd_name` / paths / `adb_serial` kept but `notes` and any `$HOME`-prefixed path
      segment replaced by `~`**. Returns the zip path; the UI then `reveal_path`s it.
- [ ] Redaction test (Rust): a log line containing a home path and a token-looking string comes out
      with the home path collapsed to `~` (tokens: best-effort — document what is / isn't scrubbed).
- [ ] "Export diagnostics" button on the Dependencies screen → `export_diagnostics` → `reveal_path`
      the result. Vitest: the button calls the command.
- [ ] `just bindings` clean; `just check-fast` then `just validate` green.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: `tracing-appender` non-blocking guard lifetime — it has to outlive `run()`; the exact
redaction rules and their limits; the zip contents; whether diagnostics goes in `commands/host.rs`
or its own module.)
