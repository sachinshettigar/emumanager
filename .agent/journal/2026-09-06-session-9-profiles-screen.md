# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- Task `0024` (done): Profiles screen + Export-profile + Save-as-profile. **M4 functionally
  complete** (`0022`–`0024` all done).
- `currentMilestone` → M5.

## Changed

- `src/lib/ipc.ts` — 7 profile hooks (`useInspectProfile` / `useApplyProfile` / `useExportProfile`
  / `useProfiles` / `useSaveProfile` / `useDeleteProfile` / `getSavedProfile`).
- `src/routes/Profiles.tsx` (rewritten) + `Profiles.test.tsx` (new, 4 tests).
- `src/routes/EmulatorDetail.tsx` — "Export profile" button (`useExportProfile`, clipboard +
  `<textarea>`).
- `src/routes/Create.tsx` — "Save as profile" in the review step (client-side `EmuProfile` build).
- `MILESTONES.md` (M4 boxes), `PROGRESS.md`, `.agent/state.json`, `.agent/tasks/0022`–`0024`.

## State now

- `just validate`: **pass** (131 rust tests, 31 web tests).
- `just progress`: pass (milestone M5).
- Tasks `0022`–`0024` `done`. M4 `in_progress` on the DoD E2E (→ M6).
- `lastValidatedCommit`: set after pushing.

## Next action

Scope **M5 — Host readiness & elevated helper** into task files. From `MILESTONES.md` M5:
- `emu-host` per-OS detection: virtualization (cpuid / registry / sysctl), accelerator kind +
  status, disk, RAM → `HostReport` + `verdict` + `fixes[]`. (`emu_core::model::host` already has
  `HostReport` / verdict / fixes types — task `0002`.)
- `emu-helper` binary: `check`, `enable-whpx`, `enable-aehd`, `add-kvm-group` → structured JSON.
  (The CLI stubs exist from task `0001`, emitting `not_implemented` JSON.)
- `run_helper(fix)` — invoke `emu-helper` with OS elevation (UAC / `pkexec` / `sudo`), re-probe.
- Dependencies-screen host panel: live tiles, per-fix buttons, reboot/firmware checklist.
- Graceful degradation: no accelerator → still list / create, warn about speed, block launch with
  a clear reason + link to the fix.

Suggested split: `0025` `emu-host` detection (per-OS probes behind the `HostProbe` port, fully
unit-tested with fake sysctl/registry/cpuid inputs), `0026` `emu-helper` real subcommands +
`run_helper` IPC + the `HostProbe` wiring, `0027` the Dependencies host panel + launch-gating.

## Gotchas / notes for the next agent

- **M4's `sanitize_avd_name` is in two places** (`emu_core::profile` + `commands/emulator.rs`) —
  still a pending cleanup.
- **`parse_profile` doesn't run the JSON Schema** — `bad-abi.json` passes. Frontend `ajv` is the
  schema gate; the Profiles screen currently just surfaces whatever `inspect_profile` returns.
- **`jsonschema` is a dev-dep only** (`default-features = false`). Don't pull it into `src-tauri`.
- **`emu-host` has no `src-tauri` consumer yet** — it's on `cargo-machete`'s `ignored` list in
  `src-tauri/Cargo.toml`. M5 removes it from that list once wired.
- CI still blocked on GitHub billing.
