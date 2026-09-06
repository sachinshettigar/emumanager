---
id: "0022"
title: "Profile engine — resolve() + RequirementDiff + export + schema-sync test"
milestone: "M4"
status: "review"
owner: "Claude Code"
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

The pure half of M4, all in `emu-core`, no IPC: turn an `EmuProfile` into a `RequirementDiff`
against local install state (`resolve()`), turn a tracked emulator or a `CreateSpec` back into an
`EmuProfile` (`export`), and lock the serde model to `schemas/emuprofile/v1.schema.json` with a
test that validates the model's own round-tripped JSON against the schema.

## Context / links

- Milestones: `MILESTONES.md` M4 bullets 1, 3 (the resolve half), 7 (round-trip — the pure part)
- Architecture: `docs/architecture.md` §3 "Profile engine", §5 "Import profile"
- ADR: `docs/adr/0004` — a profile is a recipe, never carries SDK / image bytes
- Existing: `crates/emu-core/src/model/profile.rs` (`EmuProfile` + `From` conversions already there),
  `crates/emu-core/src/model/plan.rs` (`Plan` / `Requirement` / `RequirementKind` / `RequirementStatus`
  / `CreateSpec` — reuse these), `crates/emu-core/src/toolchain::InstalledState`

## Scope — files this task may touch

- `crates/emu-core/src/model/profile.rs` (`EmuProfile::from` an emulator / a `CreateSpec`; a
  `to_json_pretty` helper; the schema-sync test)
- `crates/emu-core/src/profile/mod.rs` + `crates/emu-core/src/profile/resolve.rs` (new module —
  `resolve(profile, installed, image_catalog_size_lookup) -> RequirementDiff`)
- `crates/emu-core/src/model/plan.rs` (only if `RequirementDiff` needs a small addition — prefer
  reusing `Vec<Requirement>` + a thin newtype)
- `crates/emu-core/src/lib.rs` (`pub mod profile`)
- `crates/emu-core/Cargo.toml` (a JSON-Schema validator dev-dependency — `jsonschema`, dev-only,
  for the sync test; **not** a runtime dep — runtime schema validation is the frontend's `ajv`)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `resolve` returns the existing `plan::Plan` (`{ diff: Vec<Requirement>, create_spec }`) —
      reused rather than a new `RequirementDiff` type; `Plan` already has
      `total_download_bytes()` / `is_ready()`. `Requirement.kind` uses `CmdlineTools` /
      `PlatformTools` / `Emulator` / `SystemImage` (`Platform` is **not** emitted — see Notes).
- [x] `resolve(profile: &EmuProfile, components: &InstalledState, image_installed: bool,
      image_size_bytes: Option<u64>) -> Result<Plan>` (`crates/emu-core/src/profile/resolve.rs`) —
      one `Requirement` per component, `Present` or `NeedsDownload { size_bytes }`; non-android /
      bad schema version → `Err` via `EmuProfile::validate`. `sanitize_avd_name` moved here (pub).
- [x] `EmuProfile::from_emulator(&Emulator)` + `EmuProfile::from_create_spec(&CreateSpec)` — the
      export direction; new reverse `From<ImageType>`/`From<Graphics>` impls in `model/profile.rs`;
      an adopted emulator's empty `device_profile_id` exports as `pixel_6` so the file still
      validates (documented in the fn).
- [x] Schema-sync test `model_round_trip_stays_schema_valid` — every `valid/*.json` fixture, parsed
      into `EmuProfile`, serialized back, re-validated against `v1.schema.json` with `jsonschema`
      (dev-dep, `default-features = false` — no `reqwest`). `cargo deny` (advisories/bans/licenses/
      sources) all pass with it.
- [x] Export round-trip test `export_from_an_emulator_round_trips_the_recipe_fields`.
- [x] 7 new tests (5 in `profile::resolve`, 2 in `model::profile`); `just check-fast` then
      `just validate` green; `emu-core` still `tauri`-free.
- [x] `docs/architecture.md` §3 profile-engine note updated (module path, `jsonschema` test,
      the "no `Platform` requirement" call-out).

## Validate

```
cargo test -p emu-core --all-features
just validate
```

## Notes / findings

### Reused `Plan`, no new `RequirementDiff` type

`plan::Plan { diff, create_spec }` already carries exactly what `resolve` needs to return and has
`total_download_bytes()` / `is_ready()`. A newtype would just wrap it. `resolve` returns `Plan`
directly — `apply_profile` (task `0023`) runs `plan.create_spec` after satisfying `plan.diff`.

### No `Platform` requirement

Task `0019`'s real capture: `avdmanager create avd -n … -k system-images;android-24;… -d pixel_6`
**succeeded before `platforms;android-24` was installed** (the platform was only installed later, to
check the `Target:` line). So `platforms;android-NN` is not a create-time dependency and `resolve`
doesn't emit a `Requirement` for it. `RequirementKind::Platform` stays in the enum for a future
need; `component_label` handles it.

### `image_installed` / `image_size_bytes` are caller-supplied

`emu-core` can't see the SDK filesystem or the `sys-img2-3.xml` catalog. `resolve` takes the two
facts as plain args. Task `0023`'s `inspect_profile` fills them: `provider.is_image_installed(coord)`
for the bool, and a lookup over the parsed `sys-img2-3.xml` entries for the size.

### `jsonschema` dev-dep

`jsonschema = { version = "0.33", default-features = false }` (0.33 is the last release supporting
rustc 1.82; 0.54 needs 1.85). `default-features = false` drops `reqwest`/`resolve-http` — the
schema is a local file with only internal `$ref`s. Dev-only; runtime schema validation stays the
frontend's `ajv`. `cargo deny check` (advisories/bans/licenses/sources) passes with the ~17 new
transitive dev deps it pulls (icu, fancy-regex, num-bigint…).

### `emu_core::profile` vs `emu_core::model::profile`

`model::profile` is the data (`EmuProfile` + sections). `profile` is the engine (`resolve`,
`sanitize_avd_name`). Same leaf name at two path levels — unambiguous, and it keeps `model/` pure
data per `lib.rs`'s own layout note. `sanitize_avd_name` is duplicated in
`src-tauri/src/commands/emulator.rs`'s `sanitize_avd_name` for now — a later cleanup can point that
at `emu_core::profile::sanitize_avd_name`.
