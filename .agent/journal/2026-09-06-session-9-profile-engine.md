# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- M3 closed out — tasks `0018`–`0021` → `done`; M3 stays `in_progress` on the M6 E2E line;
  `currentMilestone` → M4.
- M4 (Profiles) scoped into `0022`–`0024`.
- Task `0022` (done, in review): the pure profile engine in `emu-core`.

## Changed

- `crates/emu-core/src/profile/{mod,resolve}.rs` (new) — `resolve()` → `Plan`; `sanitize_avd_name`.
- `crates/emu-core/src/model/profile.rs` — reverse `From` impls, `from_emulator` /
  `from_create_spec` / `to_json_pretty`, schema-sync + export-round-trip tests.
- `crates/emu-core/src/lib.rs` — `pub mod profile`.
- `crates/emu-core/Cargo.toml` — `jsonschema` dev-dep (`default-features = false`).
- `docs/architecture.md` §3 profile-engine note.
- `MILESTONES.md`, `PROGRESS.md`, `.agent/state.json`, `.agent/tasks/0018`–`0022`.

## State now

- `just validate`: **pass** (`cargo deny` incl.). `just progress`: pass (milestone M4).
- Rust tests: +7 (profile engine + schema-sync + export round-trip).
- Tasks: `0018`–`0021` done, `0022` review, `0023`–`0024` todo.
- `lastValidatedCommit`: set after pushing.

## Next action

Task `0023` — Profile IPC (`src-tauri/src/commands/profile.rs`):
- `inspect_profile(bytes) -> ProfileInspection` — `serde_json::from_slice` → `EmuProfile::validate`
  → `profile::resolve(&profile, &InstalledState, provider.is_image_installed(coord).await?,
  size_from_catalog(coord))`. Specific rejection messages: not-JSON → "not a .emuprofile"; wrong
  `platform` → "targets <x>, not Android"; wrong `schemaVersion` → "unsupported .emuprofile
  version <v>".
- `apply_profile(bytes, launch) -> String` — reuse `create_emulator`'s flow (ensure_image →
  create → optional launch) but stamp `EmulatorSource::Imported`. Needs a provider `create` that
  takes a `source`, or a post-create registry update.
- `export_profile(id) -> String` — `EmuProfile::from_emulator(&<row→Emulator>).to_json_pretty()`.
- Saved profiles: `migrations/0003_profiles.sql` adds a `json TEXT` column to `profiles`;
  `Registry::save_profile` / `list_profiles` / `get_profile` / `delete_profile`.

## Gotchas / notes for the next agent

- **`resolve` returns `plan::Plan`**, not a bespoke diff type. `plan.diff` is `Vec<Requirement>`,
  `plan.create_spec` is ready to run. `Requirement.status` is `Present | NeedsDownload { size_bytes }`.
- **`image_size_bytes` for `inspect_profile`**: parse the six `sys-img2-3.xml` manifests (same as
  `list_images` does), find the entry whose `coord == profile.image_coord()`, use `entry.size_bytes`.
- **No `Platform` requirement** — deliberate (task `0022` Notes). Don't add one back without a real
  capture showing `avdmanager create` needs `platforms;android-NN`.
- **`EmulatorSource::Imported`** needs `profile_id` + `origin_label`; `AndroidProvider::create`
  currently hard-codes `Manual { discovered: false }`. Either add a `source` param to `create` or
  do `registry.upsert_emulator` with the right source right after (simpler, in the command).
- **`sanitize_avd_name` is now in two places** — `emu_core::profile::sanitize_avd_name` and
  `commands/emulator.rs`. `apply_profile` should use the emu-core one; a later pass can delete the
  duplicate.
- **`jsonschema` is dev-only** — do not reach for it at runtime in `src-tauri`; the frontend's
  `ajv` (and `EmuProfile::validate` server-side) cover runtime validation.
- CI still blocked on GitHub billing.
