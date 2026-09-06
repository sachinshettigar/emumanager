---
id: "0022"
title: "Profile engine — resolve() + RequirementDiff + export + schema-sync test"
milestone: "M4"
status: "doing"
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

- [ ] `RequirementDiff` — a `Vec<Requirement>` (reuse `plan::Requirement`) wrapped so it can carry
      `total_download_bytes()` / `is_ready()` helpers; `Requirement.kind` covers `SystemImage`,
      `Platform` (the `platforms;android-NN` matching the API), `Emulator`, `PlatformTools`,
      `CmdlineTools`.
- [ ] `resolve(profile: &EmuProfile, installed: &InstalledState, size_of: impl Fn(ImageCoord) -> Option<u64>)
      -> Result<RequirementDiff>` — one `Requirement` per component the profile needs, each
      `Present` or `NeedsDownload { size_bytes }`; a non-android / bad-schema-version profile is an
      `Err` (reuse `EmuProfile::validate`).
- [ ] `EmuProfile::from_emulator(&Emulator) -> EmuProfile` and
      `EmuProfile::from_create_spec(name, &CreateSpec) -> EmuProfile` — the export direction; the
      known schema/CLI naming drift (module doc: `android-automotive_playstore` vs `-playstore`,
      graphics `hardware` vs `host`) is handled by the existing `From` impls in reverse.
- [ ] Schema-sync test: for every `schemas/emuprofile/fixtures/valid/*.json`, parse → re-serialize
      → JSON-Schema-validate the re-serialized JSON against `v1.schema.json` with `jsonschema`
      (dev-dep). Catches any model field that doesn't round-trip to a schema-valid shape.
- [ ] Export round-trip test: `EmuProfile` → `from_emulator` of an `Emulator` built from it →
      compare the recipe-relevant fields (device, image, hardware) are equal.
- [ ] Tests added for the above; `just check-fast` then `just validate` green; `emu-core` still
      `tauri`-free.
- [ ] Docs: `docs/architecture.md` §3 profile-engine note updated if the module layout differs.

## Validate

```
cargo test -p emu-core --all-features
just validate
```

## Notes / findings

(Fill in: `RequirementDiff` shape decision; how `size_of` is supplied — the caller (M4 task 0023)
passes a closure over the `sys-img2-3.xml` catalog; the `Platform` requirement — is
`platforms;android-NN` actually needed for `avdmanager create`? verify against the real capture
from task 0019's scratch SDK, or note the open question.)
