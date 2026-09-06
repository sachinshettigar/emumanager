# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- Task `0020` (done, in review): shared managed `AndroidProvider` + startup reconcile + exit reap
  + the rename / edit-hardware / delete / wipe / detail / reveal_path commands.
- Milestone: `M3`.

## Changed

- `src-tauri/src/provider_state.rs` (new) — `ManagedProvider` (`OnceCell<Arc<AndroidProvider>>` +
  `(data_dir, os)`).
- `src-tauri/src/lib.rs` — `mod provider_state`; `setup` manages it + spawns a startup
  `reconcile()`; `run()` switched to `.build(ctx).run(|handle, event| …)` with a
  `RunEvent::ExitRequested` reap; 7 new commands registered (16 total).
- `src-tauri/src/commands/emulator.rs` — all commands take `State<'_, ManagedProvider>` (old
  per-call `provider()` helper gone); new `reconcile_now`, `rename_emulator`, `edit_hardware`,
  `delete_emulator`, `wipe_emulator_data`, `emulator_detail` (+ `EmulatorDetail` DTO), `reveal_path`.
- `crates/emu-android/src/provider.rs` — new pub methods `detail`, `rename`, `set_hardware`,
  `wipe_data`, `shutdown`.
- `src/lib/bindings.ts` — regenerated (16 commands + `EmulatorDetail`).
- `src/lib/ipc.ts` — `useReconcileNow`; `src/routes/Dashboard.tsx` — "Refresh" button;
  `src/routes/Dashboard.test.tsx` — a test for it (23 web tests).
- `MILESTONES.md` (M2 orphan gap resolved, M3 bullets), `PROGRESS.md`, `.agent/state.json`,
  `.agent/tasks/0020`, `.agent/tasks/0021` (note the shared backend).

## State now

- `just validate`: **pass** (128 rust tests, 23 web tests). The `bindings.ts` diff is the usual
  pre-commit regen the lefthook stages.
- `just progress`: pass (milestone M3, 21 tasks / 21 files).
- Tasks moved: `0020` todo→review.
- `lastValidatedCommit` in state.json: set after pushing.

## Next action

Task `0021` — `/emulator/:id` detail panel + per-emulator log console. Backend commands for the
panel (`emulator_detail`, `rename_emulator`, `edit_hardware`, `delete_emulator`,
`wipe_emulator_data`, `reveal_path`) and the `EmulatorDetail` type are **already done and in
`bindings.ts`** — `0021` adds their `ipc.ts` hooks (held out of `0020` so `knip` wouldn't flag
them) plus the route. For the log console: tee `AndroidProvider::launch`'s streamed output to
`<data_dir>/logs/<avd_name>.log` (one code path, not a second drain), add an
`emulator_log_tail(id, max_lines)` command, and reuse the `job://emulator` `Log` event for the
live stream.

## Gotchas / notes for the next agent

- **`ManagedProvider::get()` builds on first call** (`OnceCell::get_or_try_init`). `setup` only
  records `(data_dir, os)`; if either is `None` (unsupported OS / no data dir) `get()` returns an
  `unsupported` `IpcError` and the app still runs. `peek()` (no build) is for the exit reaper only.
- **`edit_hardware` records intent only** — updates `hardware_json`, does not touch the live AVD
  `config.ini` or recreate the AVD, so RAM/storage changes don't apply until the AVD is next
  recreated. The command + `AndroidProvider::set_hardware` docs say so. Recreate-on-edit is a
  future task.
- **`wipe_emulator_data`** deletes `userdata-qemu.img` (+`.qcow2`), `cache.img`, `snapshots/` etc.
  from `$ANDROID_AVD_HOME` / `~/.android/avd` / `<avd>.avd` via `Fs`; next launch rebuilds them.
  Refuses while running. There is no captured fixture for the exact file set — it's the documented
  `-wipe-data` mechanism; adjust `WIPE` in `AndroidProvider::wipe_data` if a real run shows more.
- **`reveal_path`** is a bare `std::process::Command` spawn (no `tauri-plugin-opener`). Linux has
  no portable reveal-and-select so it opens the containing directory.
- The detail-panel hooks were deliberately NOT added to `ipc.ts` in `0020` — `knip` fails
  `just validate` on an exported-but-unimported symbol. Add them in `0021` next to the panel.
- CI still blocked on GitHub billing.
