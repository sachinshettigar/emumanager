# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- Task `0023` (done, in review): Profile IPC — inspect / apply / export + saved-profiles registry.
- Milestone: `M4`.

## Changed

- `migrations/0003_profiles.sql` (new) — `json` + `description` columns on `profiles`.
- `crates/emu-core/src/registry/open.rs` — `save_profile` / `list_profiles` / `get_profile` /
  `delete_profile` / `set_source`; `saved_profiles_crud_and_set_source` test.
- `crates/emu-android/src/provider.rs` — `registry()`, `installed_state()`, `set_source` accessors;
  `sdk_root` refactored onto `installed_state`.
- `src-tauri/src/commands/profile.rs` (new) — the 7 profile commands + `parse_profile` + tests.
- `src-tauri/src/commands/emulator.rs` — `job_handle` / `emit_job` promoted to `pub(crate)`.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — register (24 commands).
- `src/lib/bindings.ts` — regenerated.
- `MILESTONES.md`, `PROGRESS.md`, `.agent/state.json`, `.agent/tasks/0023`.

## State now

- `just validate`: **pass** (the `bindings.ts` diff is the pre-commit regen).
- `just progress`: pass (milestone M4).
- Tasks: `0022`–`0023` review, `0024` todo.
- `lastValidatedCommit`: set after pushing.

## Next action

Task `0024` — the Profiles screen. `src/lib/ipc.ts`: `useInspectProfile` (mutation, takes
`number[]` from a `File` read), `useApplyProfile`, `useExportProfile`, `useProfiles` (query),
`useSaveProfile`, `useDeleteProfile`, `getSavedProfile`. `src/routes/Profiles.tsx` rewritten: a
drop zone / file input → `inspect_profile` → preview (name, device, image, the requirement table,
total download) + Apply (streams `useEmulatorJob`); saved-profiles list. `EmulatorDetail.tsx`: an
"Export profile" button (`export_profile` → copy JSON + a written-file `reveal_path`, or just
clipboard — pick and note). `Create.tsx` review step: "Save as profile" (`save_profile` of the
wizard selection as an `EmuProfile`). Then mark the M4 `MILESTONES.md` boxes; the export→wipe→import
E2E is deferred to M6.

## Gotchas / notes for the next agent

- **`inspect_profile` / `apply_profile` / `save_profile` take `bytes: number[]`** in `bindings.ts`
  — from the frontend, `Array.from(new Uint8Array(await file.arrayBuffer()))`.
- **`apply_profile` streams on `job://emulator`** with `jobId` `apply:<avdName>` — the Profiles
  screen can reuse `useEmulatorJob` exactly like the Create wizard does.
- **`parse_profile` does not enforce the JSON Schema's enums** (`bad-abi.json` passes) — the
  frontend must still `ajv`-validate a dropped file, or accept that a schema-invalid-but-serde-valid
  file gets a domain-level error later. For M4 the frontend just surfaces whatever
  `inspect_profile` returns.
- **`export_profile` errors** if the emulator has no known `image_coord` (an adopted AVD whose
  `config.ini` didn't parse) — the detail panel should disable "Export profile" in that case or
  show the message.
- **No `Platform` requirement** in the diff (task `0022` decision).
- CI still blocked on GitHub billing.
