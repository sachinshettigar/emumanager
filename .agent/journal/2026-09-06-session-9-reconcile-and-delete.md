# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- Task `0019` (done, in review): `AndroidProvider::reconcile` + `delete` + kill-safety + the M3
  DoD property test.
- Milestone: `M3`.

## Changed

- `crates/emu-android/src/avd_list.rs` (new) — `parse_avdmanager_list_avd(&str) -> Vec<AvdEntry>`.
- `crates/emu-android/tests/fixtures/avdmanager-list-avd.txt` (new) — real capture from
  `cmdline-tools 16111833` (see Notes in the task file for how).
- `crates/emu-android/src/lib.rs` — `pub mod avd_list`.
- `crates/emu-android/src/provider.rs` — `reconcile` + `delete` implemented (were
  `NotImplemented`); `adopt_row` (async helper, reads `config.ini` via `Fs`) + `parse_ini` (free
  fn); 11 new tests (3 in `avd_list`, 8 `provider` — adopt/flag, kill-safety, booting, delete
  wipe/untrack/running/unknown, and the 48-scenario DoD property test).
- `MILESTONES.md` (M3 bullets 2 / 4-delete / 6 / DoD), `docs/architecture.md` (reconcile note),
  `PROGRESS.md`, `.agent/state.json`.

## State now

- `just validate`: **pass** (128 rust tests, 22 web tests).
- `just progress`: pass (milestone M3, 21 tasks / 21 files).
- Tasks moved: `0019` todo→review.
- `lastValidatedCommit` in state.json: set to this session's commit after pushing.

## Next action

Task `0020` — shared managed provider + lifecycle commands. Build one `AndroidProvider` in
`src-tauri` `setup` and `app.manage()` it (construction is async — `tauri::async_runtime::block_on`
in `setup`, or a `OnceCell` on first command; pick one, note why). Switch every emulator command
off `provider(&app).await?` to the managed instance — that makes `0016`'s spawned-child map
persist, so `stop_emulator` can force-kill and a `RunEvent::ExitRequested` handler can reap every
child (kills the "app exit orphans an emulator" M2 gap). Run `reconcile()` once on startup
(spawned, non-blocking). Add commands: `rename_emulator`, `edit_hardware`, `delete_emulator(wipe)`,
`wipe_emulator_data`, `reconcile_now`, `emulator_detail` + `ipc.ts` hooks + Vitest.

## Gotchas / notes for the next agent

- **`reconcile` issues several process calls** in this order: `avdmanager list avd` (once),
  `adb devices` (once, via `emulator_serials`), `adb -s <serial> emu avd name` per running serial,
  `adb -s <serial> shell getprop sys.boot_completed` per row that's in `adb devices`. The
  fake-driven tests register `emu avd name` rules in running-serial order for that reason.
- **Kill-safety liveness = "not in `adb devices`"**, not a real pid check — the `ProcessRunner`
  port has no way to test an arbitrary pid. Fine for the dashboard's purposes; if a real pid probe
  is ever needed it's a new port method, not a tweak here.
- **`avdmanager delete avd` removes the entire `.avd` dir** (userdata included). There is no
  "keep the AVD, wipe only userdata" via avdmanager — that path is `-wipe-data` on the next
  `emulator` launch, which is task `0020`'s `wipe_emulator_data`.
- **`delete(wipe=false)` is "untrack"** — row goes, AVD stays, next `reconcile` re-adopts it as
  `Manual { discovered: true }`. That's intentional (matches `MILESTONES.md` "delete with/without
  AVD removal").
- **The scratch SDK used for the capture is in the session scratchpad** (`.../scratchpad/sdk`,
  ~4 GB with the android-24 image + emulator). Not referenced by anything in the repo; safe to
  ignore / let the session clean up.
- **`avdmanager list avd -c`** would be simpler to parse but silently omits un-loadable AVDs —
  don't switch `reconcile` to it.
- CI still blocked on GitHub billing.
