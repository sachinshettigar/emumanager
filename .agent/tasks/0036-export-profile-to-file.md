---
id: "0036"
title: "Export an emulator's profile to a file — parity with the drag-in import"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

Export already existed as clipboard-only, buried on the detail panel. Make it a real file and a
top-level action. Fifth of the 8-item feature batch (item #5: "add ability to export profile of
the emulator like we have created to import").

## Context / links

- User request (session 10): "add ability to export profile of the emulator like we have
  created to import."
- Import lives on `src/routes/Profiles.tsx` — a `FileReader` drop zone that reads a
  `.emuprofile`. The mirror is producing such a file.
- `export_profile` (clipboard JSON) + `save_profile` (save to the local list) already exist
  (`src-tauri/src/commands/profile.rs`, task 0023). `reveal_path` (task 0021) already exists.
- No Tauri dialog/fs plugin is wired (kept minimal per ADR 0007 / task 0028), so this writes to a
  fixed `<data_dir>/exports/` dir and reveals it, rather than a save-as dialog.

## Scope — files this task may touch

- `src-tauri/src/provider_state.rs` — `ManagedProvider::data_dir()` accessor
- `src-tauri/src/commands/profile.rs` — `build_profile_json` helper (shared) + new
  `export_profile_to_file(id) -> String` (writes `<data_dir>/exports/<avd>.emuprofile`, returns
  the path)
- `src-tauri/src/lib.rs` — register it
- `src/lib/bindings.ts` (regen), `src/lib/ipc.ts` — `useExportProfileToFile`
- `src/routes/EmulatorDetail.tsx` — "Save as .emuprofile" button next to the (renamed) "Copy
  profile" one; shows the path + "Show in folder" (`revealPath`)
- `src/routes/Dashboard.tsx` — a per-row "Export" button
- `src/routes/EmulatorDetail.test.tsx`, `src/routes/Dashboard.test.tsx` — mock + a test each
- `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `export_profile_to_file(id)` builds the same `EmuProfile` JSON as `export_profile`, writes
      it to `<data_dir>/exports/<avd_name>.emuprofile`, and returns the absolute path.
      `fs_error` on a write failure; `unsupported` when there's no data dir.
- [x] `export_profile` refactored onto the shared `build_profile_json` helper (no behaviour
      change — still returns the JSON string for the clipboard).
- [x] EmulatorDetail: "Copy profile" (was "Export profile") + "Save as .emuprofile"
      (`export-profile-file`); on success a line shows `exported-path` and a "Show in folder"
      button that calls `revealPath`.
- [x] Dashboard: each row has an "Export" button (`export-<id>`) → `export_profile_to_file`,
      label flips to "Exported ✓" with the path in the `title`.
- [x] Vitest: EmulatorDetail "saves the profile to a file and shows the path"; Dashboard
      "exports an emulator profile to a file from its row". 42 web tests.
- [x] `just validate` green apart from the expected `bindings.ts` diff step.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- **No save-as dialog.** `tauri-plugin-dialog` / `-fs` aren't wired and adding them is a
  capability + dependency change out of proportion to this task. `<data_dir>/exports/` +
  `reveal_path` gives the user a real file they can then move/share — the important half of
  parity with import. A dialog is a clean follow-up if wanted.
- `build_profile_json` returns `(json, avd_name)` — the AVD name doubles as a safe filename stem
  (it's already sanitised at create time).
