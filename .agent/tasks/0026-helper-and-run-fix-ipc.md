---
id: "0026"
title: "emu-helper real subcommands + probe_host / run_helper IPC"
milestone: "M5"
status: "review"
owner: "Claude Code"
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

- [x] `emu-helper <cmd>` prints one line of camelCase JSON `{ command, status, message, needsReboot }`,
      `status` ∈ `ok` / `failed` / `notApplicable` / `needsReboot` (`needs_reboot` set when
      `status == "needsReboot"`). Exit `0` for everything but `failed` (exit `1`). Real bodies in
      `crates/emu-helper/src/actions.rs`, `#[cfg(target_os)]`:
  - `check` — macOS `sysctl kern.hv_support`; Linux open `/dev/kvm`; Windows
    `Get-WindowsOptionalFeature`. No mutation.
  - `enable-whpx` — Windows `dism /online /enable-feature /featurename:HypervisorPlatform /all
    /norestart` → `needsReboot`; elsewhere `notApplicable`.
  - `enable-aehd` — Windows `failed` with manual steps (can't be scripted — see Notes); elsewhere
    `notApplicable`.
  - `add-kvm-group` — Linux `usermod -aG kvm <user>` where `<user>` = `$SUDO_USER` or resolved
    from `$PKEXEC_UID` (never `root`); elsewhere `notApplicable`.
- [x] Output-schema test — `crates/emu-helper/tests/schema.rs` runs each subcommand as a real
      subprocess: asserts the JSON shape, `status` ∈ the allowed set, non-empty `message`, and that
      exit code matches `status` (`failed` ⇒ non-zero).
- [x] `probe_host() -> HostReportDto` (`src-tauri/src/commands/host.rs`) — `NativeHostProbe::inspect()`
      → a flat `#[derive(specta::Type)]` DTO (enums → strings; `u64` RAM/disk → `u32` **MB**), and a
      best-effort `registry.insert_host_snapshot` with the serialized `HostReport`.
- [x] `run_helper(fixId) -> HelperOutcomeDto` — rejects non-scriptable ids (`invalid`); resolves
      `emu-helper` next to the app binary (`current_exe().parent()`); `elevated_argv(helper, sub, os)`
      (a **pure, unit-tested** fn) wraps it — macOS `osascript … "do shell script … with
      administrator privileges"`, Linux `pkexec`, Windows `powershell Start-Process -Verb RunAs
      -Wait`. A dismissed prompt (`User canceled` / `pkexec` exit 126) → `IpcError` code `cancelled`.
      macOS/Linux parse the helper's JSON line; Windows can't capture the child's stdout so it
      returns a synthetic "re-probe to see the result" outcome (documented).
- [x] Tests: `elevated_argv` per OS, `run_helper` non-scriptable rejection, `HostReportDto` mapping
      (3 in `commands::host`); the `emu-helper` schema tests (2). `emu-helper check` on this Mac:
      `{"command":"check","status":"ok","message":"Apple's Hypervisor (HVF) is available …"}`.
- [x] `just bindings` clean once committed (26 commands); `just check-fast` then `just validate`
      green (154 rust tests). Also: `emu-host` removed from `src-tauri`'s `cargo-machete` ignore
      list (now a real consumer) — the whole block is gone.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### Elevation wrapper — pure `elevated_argv`, no real elevation in tests

`elevated_argv(&Path, sub, os) -> (program, Vec<args>)` is a plain function with a per-OS `match`,
so its output is asserted directly (macOS wraps in `osascript … with administrator privileges`,
Linux is `pkexec <helper> <sub>`, Windows is `powershell Start-Process … -Verb RunAs -Wait`). The
actual `Command::output()` isn't exercised by a unit test — the real prompt is a manual /
`--ignored` affair. `run_helper`'s *rejection* path (non-scriptable id) and cancel detection
(`stderr` contains "User canceled" / exit 126) are covered.

### `emu-helper` path resolution

`std::env::current_exe().parent()` + `emu-helper[.exe]`. In a bundled Tauri app the helper is a
sibling resource next to the main binary; in `cargo run`/dev it's the sibling in
`target/<profile>/`. If it isn't found → `not_found` `IpcError` (the panel disables the "Fix it"
button). No `target/` walk — the sibling covers both cases.

### `enable-aehd` is guidance-only

AEHD (the HAXM successor) ships as an MSI/driver package in the SDK's
`extras;google;Android_Emulator_Hypervisor_Driver`, installed via its own `silent_install.bat`.
There's no single system command to script it cleanly, so `emu-helper enable-aehd` returns
`failed` with the manual steps in `message`. `run_helper` still offers it (a Windows AMD host with
no WHPX) but the outcome is instructions, not an action. Revisit if a scriptable path appears.

### Windows is untested

`actions.rs`'s Windows bodies (`dism`, `Get-WindowsOptionalFeature`) and `run_helper`'s
`Start-Process -Verb RunAs` path have **not** been run on a real Windows host — this is a macOS dev
machine. The `dism` invocation is the documented one; `Start-Process` genuinely can't forward the
child's stdout, hence the synthetic outcome + "re-probe". Verify both on a Windows CI runner in M6.

### `emu-helper check` on this Mac

`{"command":"check","status":"ok","message":"Apple's Hypervisor (HVF) is available — no setup needed.","needsReboot":false}`.

### `run_helper` uses `tokio::process`

`src-tauri` already has `tokio` with `process`; `run_helper` is `async` and awaits the elevated
command so a slow prompt doesn't block the command thread.
