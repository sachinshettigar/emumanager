---
id: "0026"
title: "emu-helper real subcommands + probe_host / run_helper IPC"
milestone: "M5"
status: "todo"
owner: ""
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

`emu-helper` does the real scriptable accelerator setup and prints structured JSON; `src-tauri`
gets `probe_host()` (runs `NativeHostProbe`, writes a `host_snapshots` row) and `run_helper(fixId)`
(invokes `emu-helper` with OS elevation, parses its JSON, re-probes).

## Context / links

- Depends on task `0025` (`NativeHostProbe`, `build_report`, the fix ids)
- `crates/emu-helper/src/main.rs` — clap CLI, currently every subcommand prints `not_implemented`
- Architecture: `docs/architecture.md` §5 "Host readiness / fix"; `docs/adr/0004`-adjacent
- `crates/emu-core/src/registry` — `insert_host_snapshot` / `latest_host_snapshot` exist (task `0018`)

## Scope — files this task may touch

- `crates/emu-helper/src/main.rs`, `crates/emu-helper/src/actions.rs` (new — the real per-OS
  subcommand bodies behind `#[cfg]`), `crates/emu-helper/Cargo.toml`
- `src-tauri/src/commands/host.rs` (new) — `probe_host`, `run_helper`
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (register), `src-tauri/Cargo.toml`
  (drop `emu-host` from the `cargo-machete` ignore list once it's imported)
- `src/lib/bindings.ts` (regenerated) — hooks held for task `0027`
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] `emu-helper <cmd>` prints a single-line JSON `{ command, status, message, needsReboot }`
      where `status` is `ok` / `failed` / `not_applicable` / `needs_reboot`. Real bodies:
  - `check` — report what's present + what this helper could change (no mutation).
  - `enable-whpx` (Windows) — `dism /online /enable-feature /featurename:HypervisorPlatform /all
    /norestart`; on non-Windows → `not_applicable`.
  - `enable-aehd` (Windows) — install + enable the AEHD driver (cite the real steps); non-Windows
    → `not_applicable`.
  - `add-kvm-group` (Linux) — `usermod -aG kvm $SUDO_USER` (uses `$SUDO_USER`, since it runs as
    root); non-Linux → `not_applicable`.
  - Exit code 0 on `ok` / `not_applicable` / `needs_reboot`, non-zero on `failed`.
- [ ] `emu-helper` output schema test (the M5 DoD's second half): a Rust test that runs each
      subcommand in a subprocess and asserts the JSON shape + that every `status` is one of the
      allowed values.
- [ ] `probe_host() -> HostReportDto` — `NativeHostProbe::inspect()`, mapped to a
      `#[derive(specta::Type)]` DTO (`u64` disk/RAM → `u32` MB or a string, per the specta bigint
      rule), and `registry.insert_host_snapshot` with the serialized report.
- [ ] `run_helper(fixId: String) -> HelperOutcomeDto` — resolves `emu-helper`'s path (next to the
      app binary, or `target/…` in dev), invokes it **with elevation** (`osascript … with
      administrator privileges` on macOS, `pkexec` then `sudo` fallback on Linux, a
      `runas`/ShellExecute `verb=runas` on Windows), parses stdout JSON, returns it. A user who
      cancels the elevation prompt → a clean `cancelled` error, not a crash.
- [ ] Tests: `run_helper` argv/elevation-wrapper construction (pure helper, fake runner);
      `probe_host` DTO mapping.
- [ ] `just bindings` clean once committed; `just check-fast` then `just validate` green. Frontend
      hooks deferred to task `0027`.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: the elevation wrapper per OS and how it's tested without actually elevating; how
`emu-helper`'s path is resolved in dev vs a bundled app; whether `enable-aehd` can be scripted at
all or is guidance-only; what `emu-helper check` reported on this Mac.)
