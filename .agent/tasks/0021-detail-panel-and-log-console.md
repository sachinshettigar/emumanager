---
id: "0021"
title: "Emulator detail panel + per-emulator log console"
milestone: "M3"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
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

- [ ] `launch` writes every streamed line to `<data_dir>/logs/<avd_name>.log` (created if missing,
      truncated at launch start) as well as onto the job — one code path, tee not duplicate-drain.
- [ ] `emulator_log_tail(id, max_lines) -> Vec<String>` reads the tail of that file; empty (not an
      error) when the emulator has never been launched.
- [ ] `/emulator/:id` route: `emulator_detail` data rendered — display name (inline-editable →
      `rename_emulator`), device profile, image coord + API + Play Store flag, RAM / storage
      (editable → `edit_hardware`), source, live state + adb serial + gRPC port, AVD folder path with
      an "Open folder" button (`@tauri-apps/plugin-opener` or a `reveal_path` command — pick one, note it).
- [ ] Wipe data / Delete (with a "also remove the AVD" checkbox) / Stop buttons, each wired and each
      with a confirm step for the destructive ones.
- [ ] Log console: shows `emulator_log_tail` on mount, then appends live `EmulatorJob` `Log` lines
      for that emulator while it's booting/running. Copy-to-clipboard; "open log file" button.
- [ ] Dashboard rows link to the detail route.
- [ ] Vitest: detail renders from mocked bindings, rename calls the command, log tail + a live
      appended line both show. Rust: the log-file tee is covered (`InMemoryFs`).
- [ ] `just bindings` clean once committed; `just check-fast` then `just validate` green

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: tee implementation without an `Fs::append` port change if possible; opener plugin vs a
`reveal_path` command; how the live log is filtered to one emulator — jobId convention.)
</content>
