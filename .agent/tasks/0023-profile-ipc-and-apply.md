---
id: "0023"
title: "Profile IPC — inspect / apply / export / saved-profiles registry"
milestone: "M4"
status: "todo"
owner: ""
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

Wire task `0022`'s engine to the app: `inspect_profile(bytes)` (parse + validate + resolve →
diff), `apply_profile(...)` (ensure image → create with `source: Imported` → optional launch),
`export_profile(id) -> String`, and a saved-profiles list backed by the `profiles` table.

## Context / links

- Depends on task `0022` (`resolve`, `RequirementDiff`, `EmuProfile::from_*`)
- `src-tauri/src/commands/emulator.rs` (pattern for provider-driven commands, the `EmulatorJob` event)
- `crates/emu-android/src/provider.rs` — `ensure_image` + `create` already exist; `apply` is those
  two plus setting `EmulatorSource::Imported`
- `migrations/0001_init.sql` `profiles(name, source_path, created_at)` — extend if needed
- `crates/emu-core/src/registry` — add `save_profile` / `list_profiles` / `get_profile` / `delete_profile`

## Scope — files this task may touch

- `crates/emu-core/src/registry/{open.rs,row.rs,mod.rs}` (profile rows — store the profile JSON)
- `migrations/0003_profiles.sql` (new — add a `json` column to `profiles`, or a `profiles` rework)
- `crates/emu-android/src/provider.rs` (`apply_profile` helper, or a thin `create` variant that
  stamps `EmulatorSource::Imported { profile_id, origin_label }`)
- `src-tauri/src/commands/profile.rs` (new) — `inspect_profile`, `apply_profile`, `export_profile`,
  `save_profile`, `list_profiles`, `delete_profile`
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (register)
- `src/lib/bindings.ts` (regenerated), `src/lib/ipc.ts` (hooks — with a consumer, or held for 0024)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] `inspect_profile(bytes: Vec<u8>) -> ProfileInspection` — parse JSON → `EmuProfile::validate`
      → `resolve()` against real `InstalledState` + a size lookup over the `sys-img2-3.xml` catalog.
      `ProfileInspection { name, description, deviceLabel, imageLabel, requirements: [...], totalDownloadBytes, ready }`.
      A non-JSON / non-android / unknown-schema input returns a **specific** `IpcError` message
      (not "invalid") — e.g. "not a .emuprofile (missing schemaVersion)", "profile targets iOS, not
      Android", "unsupported .emuprofile version 2.0".
- [ ] `apply_profile(bytes, launch: bool) -> String` (new emulator id) — ensure the image (streams
      on `job://emulator`, `jobId` `apply:<name>`), `create` with
      `source: Imported { origin_label: "imported .emuprofile" }`, optional launch. Reuses the
      `create_emulator` machinery.
- [ ] `export_profile(id) -> String` — the emulator's `EmuProfile` as pretty JSON.
- [ ] Saved profiles: `save_profile(bytes)` (validates, stores the JSON + name), `list_profiles() ->
      [ProfileSummary]`, `delete_profile(name)`. `migrations/0003_*.sql` if the table needs a JSON column.
- [ ] Tests: `inspect_profile` on each `schemas/emuprofile/fixtures/{valid,invalid}` fixture
      (valid → a diff, invalid → the right specific message); `export_profile` round-trips via
      `inspect_profile`; registry profile CRUD.
- [ ] `just bindings` clean once committed; `just check-fast` then `just validate` green.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: `profiles` table shape; the specific rejection messages; whether `apply_profile` shares
code with `create_emulator` or duplicates the thin flow; the size-lookup source.)
