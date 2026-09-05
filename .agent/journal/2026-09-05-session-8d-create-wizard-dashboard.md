# 2026-09-05 — session 8 (continued) (Claude Code)

## Worked on

- Task `0017` (done): IPC commands + Create wizard + Dashboard wired to the real
  `AndroidProvider`. **M2 is now functionally complete** (all of `0014`–`0017` done);
  `currentMilestone` advanced to **M3**.

## Changed

- `src-tauri/src/commands/emulator.rs` (new) — `list_devices` / `list_images` / `list_emulators`
  / `create_emulator` / `launch_emulator` / `stop_emulator` + the `EmulatorJob` event
  (`job://emulator`).
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — register the above.
- `src/lib/ipc.ts` — `useDevices` / `useImages` / `useEmulators` / `useCreateEmulator` /
  `useLaunchEmulator` / `useStopEmulator` / `useEmulatorJob`.
- `src/lib/bindings.ts` — regenerated (`+111` lines; the pre-commit hook stages it).
- `src/routes/Create.tsx` — rewritten as a 4-step wizard; `src/routes/Create.test.tsx` (new).
- `src/routes/Dashboard.tsx` — rewritten to a polled emulator list; `src/routes/Dashboard.test.tsx`
  — rewritten.
- `crates/emu-android/src/devices.rs` — `parse_from_jar`; `crates/emu-android/Cargo.toml` — `zip`.
- `crates/emu-android/src/provider.rs` — `sdk_root` made `pub`, `is_image_installed`,
  `tracked_states()` + `TrackedEmulator`.
- `crates/emu-core/src/registry/open.rs` — `list_emulators()`.
- `.agent/state.json` (M3, all M2 tasks done), `MILESTONES.md`, `PROGRESS.md`.

## State now

- `just validate`: **pass** (114 Rust tests, 22 web tests). The `bindings.ts` diff is the normal
  "regenerate before commit" state — the lefthook `pre-commit` hook stages it.
- `just progress`: pass (milestone M3, 17 tasks / 17 files)
- `lastValidatedCommit`: set to this session's commit after pushing

## Next action

Scope M3 — Registry & reliable tracking. No `.agent/tasks/` files for it yet. `MILESTONES.md` M3
lists: full SQLite schema (emulators/images/profiles/jobs/host_snapshots) + migrations;
`reconcile()` on startup and on demand; a selected-emulator detail panel; wipe / delete / rename /
edit-hardware; a per-emulator log console (history tail + live stream); kill-safety (SIGKILL
mid-boot → next start reconciles to a correct state). Two things that naturally belong here and are
called out as M2 gaps: a **shared/managed `AndroidProvider`** (so its child-handle map persists →
`stop` can force-kill and the app reaps children on exit — "app exit doesn't orphan"), and the
**launch timestamp** the Dashboard needs to show uptime.

## Gotchas / notes for the next agent

- **`AndroidProvider` is currently built per command** in `commands/emulator.rs` — the child map
  from task `0016` is empty every call, so `stop`'s force-kill fallback never fires from the UI
  (only graceful `adb emu kill`). Making the provider a `tauri::State` (async-constructed in
  `setup`) is the M3 fix and unblocks real kill-safety.
- **`Provider::list_devices` / `list_images` are still `NotImplemented`** — the real logic lives in
  the `commands::emulator` command (shell fetches manifests + reads the jar; `emu-android` only
  provides the pure parsers). If M3 adds a `Downloader` to the provider, revisit whether those
  belong on the trait.
- **`devices::parse_from_jar` tries three jar paths** (`commands::emulator::read_sdklib_jar`) —
  the `sdklib` jar's location has moved across `cmdline-tools` revisions. If a user reports "no
  devices", check whether their revision uses a fourth path.
- **`f32` → `number | null` in the generated bindings** (specta-typescript 0.0.12). Any new `f32`
  DTO field is nullable on the TS side; narrow to a formatted string if that matters.
- **`ensure_image` progress is one coarse log line**, not a byte bar — a 1.5 GB Play Store image
  download shows "this can take several minutes" then nothing until done. Streaming `sdkmanager`'s
  own progress (spawn + parse its stdout) is the follow-up.
- CI still blocked on GitHub billing — nothing to do until a human fixes it.
