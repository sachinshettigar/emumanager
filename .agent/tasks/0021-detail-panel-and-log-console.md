---
id: "0021"
title: "Emulator detail panel + per-emulator log console"
milestone: "M3"
status: "review"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-06"
---

## Goal

A selected-emulator screen: its full config (image, hardware, source, serial, gRPC port, AVD path),
"open folder", and inline rename / wipe / delete / edit-hardware wired to task `0020`'s commands.
Plus a per-emulator log console — history tail of the launch log file + a live stream while it runs.

## Context / links

- Milestones: `MILESTONES.md` M3 bullets 3 (detail panel) + 5 (log console)
- Spec: `docs/spec.md` §5.3 (detail view)
- Depends on task `0020` — the backend commands (`emulator_detail`, `rename_emulator`,
  `edit_hardware`, `delete_emulator`, `wipe_emulator_data`, `reveal_path`) and the `EmulatorDetail`
  type are **already done and in `bindings.ts`**; this task adds the thin `ipc.ts` hooks for them
  (deferred out of `0020` so `knip` wouldn't flag them as unused) alongside the panel that calls them.
- `crates/emu-android/src/provider.rs::launch` already streams emulator output onto the `JobHandle`;
  this task also **tees it to a file** so there's history to tail after a reload.

## Scope — files this task may touch

- `crates/emu-android/src/provider.rs` — tee `launch`'s output stream to `<data_dir>/logs/<avd_name>.log` via `Fs`
- `crates/emu-core/src/ports.rs` / `testing/` — only if `Fs` needs an append method (note it; prefer read-modify-write to avoid a port change)
- `src-tauri/src/commands/emulator.rs` — `emulator_log_tail(id, max_lines)` command; reuse the `EmulatorJob` `Log` event for the live stream (already keyed by `jobId`)
- `src-tauri/src/lib.rs` (register the command)
- `src/lib/bindings.ts` (regenerated), `src/lib/ipc.ts`
- `src/routes/EmulatorDetail.tsx` (new), `src/routes/EmulatorDetail.test.tsx` (new)
- `src/routes/Dashboard.tsx` (row → link to `/emulator/:id`), `src/main.tsx` / router config, `src/nav.ts` if needed
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `launch` tees every streamed line into `<data_dir>/logs/<avd_name>.log` (truncated at launch
      start) **and** onto the job — one helper, `AndroidProvider::emit_log`, called from `launch`'s
      framing lines and `wait_for_boot`'s per-line drain (not a second drain of the child).
- [x] `AndroidProvider::read_log_tail(id, max_lines)` + `emulator_log_tail(id, max_lines) -> Vec<String>`
      command — tail of that file, `max_lines` capped from the end; empty (not an error) when never
      launched; `NotFound` for an unknown id.
- [x] `/emulator/:id` route (`src/routes/EmulatorDetail.tsx`): `emulator_detail` data rendered —
      inline-editable name → `rename_emulator`; device profile; image coord + API + Play Store flag;
      RAM / storage / graphics form → `edit_hardware` (with "applied on next AVD recreate" note);
      source; live state + adb serial; created/updated; AVD folder path + "Open folder" →
      **`reveal_path` command** (chose it over `@tauri-apps/plugin-opener` — one `std::process`
      spawn, no plugin dep / capability entry). gRPC port shown as `—` (not parsed yet — same as
      `RunningHandle.grpc_port`).
- [x] Wipe data / Delete (with an "also remove the AVD" checkbox) / Launch-or-Stop buttons — Delete
      has an inline confirm step; Wipe/Delete are disabled while running.
- [x] Log console: `emulator_log_tail` (500 lines) on mount, then appends live `EmulatorJob` `Log`
      lines (from `useEmulatorJob`, deduped against the history tail). Copy-to-clipboard + "Open AVD
      folder". See Notes on the jobId-filtering simplification.
- [x] Dashboard rows link to `/emulator/:id` (`detail-link-<id>`).
- [x] Vitest (`EmulatorDetail.test.tsx`, 4 tests): renders config + live state, rename calls the
      command, log tail shows in the console, load failure surfaced. Rust (3 tests): the log tee
      round-trips through `read_log_tail`, empty for a never-launched emulator, `NotFound` for an
      unknown id.
- [x] `just bindings` clean once committed; `just check-fast` then `just validate` green (131 rust
      tests, 27 web).

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### Log tee — no `Fs::append` port change

`Fs` has `read` / `write_atomic` but no append. `emit_log` does a read-modify-write per line
(`fs.read().unwrap_or_default()` → push line + `\n` → `write_atomic`). O(n²) over the run, but a
boot log is a few hundred lines / a few KB — fine. A real streaming `Fs::append` is the follow-up
if per-emulator logs ever get large (a continuous post-boot stream would need it). `launch`
truncates the file with `write_atomic(&log_path, b"")` at the start of each run.

### `reveal_path`, not `@tauri-apps/plugin-opener`

One `std::process::Command` spawn (`open -R` / `explorer /select,` / `xdg-open <dir>`), no plugin
dependency, no `capabilities/default.json` entry. Linux has no portable reveal-and-select so it
opens the containing directory. It's a `#[tauri::command] pub fn` (sync — just a spawn).

### Live log is not filtered by jobId

`useEmulatorJob` accumulates every `job://emulator` `Log` line for the session regardless of
`jobId` (the same "only one job at a time" simplification `job://bootstrap` uses). The console
shows `[...historyTail, ...liveLines.filter(not in historyTail)]`. In practice the only live job
while you're on a detail page is that emulator's launch, so this is right; a stray `create:` line
from a concurrent wizard run in another view would also show, which is acceptable for M3. Proper
per-id filtering (thread the emulator id through the event payload) is a small follow-up.

### gRPC port still `—`

`RunningHandle.grpc_port` is `None` (never parsed from the emulator log — noted since task `0016`).
The detail panel shows `—` rather than hiding the row, so it's visible where it'll be filled in.

### Rust test — forcing the drain

`launch_tees_output_to_a_log_file...` scripts `getprop sys.boot_completed` as `["0", "1"]` so the
poll loop iterates and drains the child's scripted lines before the stream-EOF branch runs the
final boot check. With a single `"1"` the launch returns on the first poll and never reads a line
(realistic, but not what this test is exercising).
</content>
