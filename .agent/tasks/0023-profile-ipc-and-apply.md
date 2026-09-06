---
id: "0023"
title: "Profile IPC — inspect / apply / export / saved-profiles registry"
milestone: "M4"
status: "review"
owner: "Claude Code"
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

- [x] `inspect_profile(bytes) -> ProfileInspection` (`src-tauri/src/commands/profile.rs`) — `parse_profile`
      (serde + `EmuProfile::validate`) → `emu_core::profile::resolve` against real `installed_state()`
      + `image_size_bytes(coord)` (fetches the six `sys-img2-3.xml` manifests). `ProfileInspection
      { name, description, deviceLabel, imageLabel, imageCoord, requirements: [{label, present,
      downloadBytes}], totalDownloadBytes, ready }`.
- [x] **Specific** rejection messages in `parse_profile`: not-JSON → "isn't valid JSON — a
      .emuprofile is a small JSON recipe"; no `schemaVersion` → "doesn't look like a .emuprofile";
      wrong version → `unsupported` "unsupported .emuprofile version …"; `platform != android` →
      `unsupported` "targets \"<x>\", not Android"; bad field → `invalid` with the serde detail.
      (Schema-enum violations like `bad-abi.json` are NOT caught here — the domain `Abi` is wider
      than the schema's two; that's the frontend's `ajv` pass. Noted in the test + code.)
- [x] `apply_profile(bytes, launch) -> String` — `ensure_image` (streams on `job://emulator`,
      `jobId` `apply:<avdName>`) → `provider.create(spec)` → `provider.set_source(id, Imported
      { profile_id: <name>, origin_label: "imported .emuprofile" })` → optional launch. Reuses
      `emulator::{job_handle, emit_job, EmulatorJobKind}` (made `pub(crate)`).
- [x] `export_profile(id) -> String` — `EmuProfile::from_emulator(&<row→Emulator>).to_json_pretty()`;
      errors if the emulator has no known image coord.
- [x] Saved profiles: `save_profile(bytes)` (validates then stores), `list_profiles() -> [ProfileSummary]`,
      `get_saved_profile(name) -> String`, `delete_profile(name)`. `migrations/0003_profiles.sql`
      adds `json` + `description` columns to `profiles`. New `Registry` methods
      `save_profile`/`list_profiles`/`get_profile`/`delete_profile`/`set_source`; `AndroidProvider`
      `registry()` / `installed_state()` / `set_source` accessors.
- [x] Tests: `parse_profile` on the valid fixtures + each specific-message path; `Registry`
      `saved_profiles_crud_and_set_source`. (2 src-tauri, 1 emu-core.)
- [x] `just bindings` clean once committed (24 commands). Frontend hooks for these deferred to
      task `0024` (knip). `just check-fast` then `just validate` green.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### `profiles` table

`migrations/0003_profiles.sql` `ALTER TABLE`s `json TEXT NOT NULL DEFAULT '{}'` and
`description TEXT NOT NULL DEFAULT ''` onto the M0 `profiles(name PK, source_path, created_at)`.
`save_profile` upserts by name; `get_profile` returns the `json` body, which `apply_profile` can be
fed back for a saved-entry "Apply".

### `apply_profile` reuses, doesn't duplicate

It builds a `CreateSpec` from the profile and calls the same `provider.ensure_image` / `create` /
`launch` the `create_emulator` command uses, sharing `emulator::{job_handle, emit_job,
EmulatorJobKind}` (promoted to `pub(crate)`). The one addition is `provider.set_source(id,
Imported{..})` after `create` — `Provider::create` doesn't take a `source`, so a follow-up write is
simpler than a trait-signature change. `EmulatorSource::Imported` needs `profile_id` +
`origin_label`; `profile_id` is the recipe `name`.

### Size lookup

`image_size_bytes(coord)` fetches the six `sys-img2-3.xml` manifests (same set `list_images` uses),
parses each, and returns the `size_bytes` of the entry whose `coord` matches. `None` on any fetch
failure — the diff then shows the image as "needs download" without a figure.

### `bad-abi.json` is not rejected by `parse_profile`

The domain `emu_core::model::image::Abi` accepts more ABIs (`armeabi-v7a`, `x86`, …) than the
schema's `["x86_64", "arm64-v8a"]`. `parse_profile` does serde + `EmuProfile::validate`, not full
JSON-Schema validation, so `bad-abi.json` (schema-invalid, serde-valid) passes here. Full schema
enforcement is the frontend's `ajv` (`scripts/validate-schema.mjs`). Documented in the test.

### No `Platform` requirement

Carried from task `0022`: `resolve` doesn't emit one, so `ProfileInspection.requirements` is
cmdline-tools / platform-tools / emulator / system-image.

### Frontend hooks deferred

`inspect_profile` … `delete_profile` have no `ipc.ts` hook yet — that + the Profiles screen is task
`0024`. Shipping unused hooks trips `knip`. The commands + DTOs are generated into `bindings.ts`.
