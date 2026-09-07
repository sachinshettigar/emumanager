# 2026-09-08 — session 10 (Claude Code)

## Worked on

- Task `0038` — live device inspector (8-item batch items #1 + #2). Last of the batch.
- Milestone: `M6`.

## Changed

- `crates/emu-android/src/provider.rs` — `logcats` `ChildHandle` map; `logcat_start` /
  `logcat_stop` / `device_facts` / `running_serial`; `DeviceFacts` struct; `parse_battery_level`
  / `parse_df_data`; `shutdown()` reaps logcats; `ChildHandle` made `pub`. +3 unit tests.
- `src-tauri/src/commands/device.rs` (new) — `DeviceLogLine` event (`device://log`),
  `start_logcat` (background drain task), `stop_logcat`, `device_facts`, `DeviceFactsDto`.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — register module + 3 commands + event.
- `src/lib/bindings.ts` regen. `src/lib/ipc.ts` — `parseLogcatLine`, `filterLogcat`,
  `LOGCAT_LEVELS`, `useStartLogcat`, `useStopLogcat`, `useLogcatStream`, `useDeviceFacts`.
- `src/routes/EmulatorDetail.tsx` — `DeviceInspector` section (facts strip + logcat viewer).
- `src/routes/EmulatorDetail.test.tsx` — mocks + "streams logcat and filters it by level and
  text".
- `MILESTONES.md` (M7 notes on what shipped early), `.agent/state.json`, `PROGRESS.md`,
  `.agent/tasks/0038-*.md`.

## State now

- `just validate`: **pass** apart from the `git diff src/lib/bindings.ts` step (new commands;
  lefthook stages the regen on commit — expected). ~170 rust tests, 44 web tests.
- `just progress`: pass (M6, 38 tasks / 38 files).
- Tasks moved: `0038` (new) todo→review.
- `lastValidatedCommit`: set after the commit.
- **The 8-item feature batch (0032–0038) is complete.**

## Next action

The 8-item batch is done. Outstanding older M6 work: task `0029` (rotating logs +
`export_diagnostics`) and `0030` (E2E harness skeleton). Plus the standing "learning document on
architecture & code" the user asked for at the end of all features — now due. And CI is still
blocked on the GitHub Actions billing issue (human action).

## Gotchas / notes for the next agent

- **The logcat drain loop lives in `commands/device.rs`, not the provider.** `logcat_start`
  returns the `ChildHandle`; the command `tokio::spawn`s the `next_line()` loop and emits
  `device://log`. `AndroidProvider` deliberately never `tokio::spawn`s.
- **`device_facts` parsers never guess.** `parse_battery_level` / `parse_df_data` and the
  `getprop` reads take the first plausible token and return `None` on any mismatch — a weird ROM
  yields a blank field, not a wrong number. No `adb` output-format was invented (AGENTS §6.2):
  `-v threadtime`, `getprop`, `dumpsys battery`'s `level:`, and POSIX `df` columns are all
  standard/documented.
- **"Networks" is only partly delivered** — model/battery/storage yes, a full connectivity read
  (`dumpsys connectivity` / `ip addr`) is version-variable with no fixture, left for M7 (noted
  in `MILESTONES.md`).
- The custom hardware/device profile editor from item #8 was **not** built — only the "group by
  device type" half (task 0037). It stays on M7.
- Pause in the viewer snapshots `lines` into `frozen` and renders that; unchecking clears
  `frozen` so the live tail resumes.
