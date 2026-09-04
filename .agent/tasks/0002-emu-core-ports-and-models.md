---
id: "0002"
title: "emu-core ports and models (no impls)"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-05"
---

## Goal

The domain types and the port traits from `docs/architecture.md` §3, plus test fakes, compiling
and unit-tested. No real behavior yet.

## Scope — files this task may touch

- `crates/emu-core/src/model/` (device, image, emulator, profile, host, job)
- `crates/emu-core/src/ports.rs` (`ProcessRunner`, `Downloader`, `HostProbe`, `Clock`, `Fs`)
- `crates/emu-core/src/provider.rs` (`Provider` trait)
- `crates/emu-core/src/error.rs` (`CoreError` via `thiserror`)
- `crates/emu-core/src/testing/` (fakes, behind a `testing` feature)
- `crates/emu-core/tests/`

## Acceptance criteria

- [x] Model structs match `docs/context/domain-model.md`; all derive
      `serde::{Serialize,Deserialize}`. **`specta::Type` deferred to task `0004`** — specta v1
      (the only stable line) has no `derive` feature and v2 is an rc that must be pinned in
      lockstep with `tauri-specta`; 0004 owns that pin and will add the derive crate-wide (a
      mechanical `#[derive(specta::Type)]` pass). Tracked below.
- [x] `ImageCoord`, `ImageType`, `Abi` with `Display`/`FromStr` producing/parsing
      `system-images;android-34;google_apis_playstore;x86_64` (7 type tags × 4 ABIs round-trip;
      6 distinct failure cases covered)
- [x] Port traits are `async` via `async-trait`; `Provider` has an object-safety test
      (`&dyn Provider`); `ProcessRunner` returns `Box<dyn ChildProcess>`
- [x] `CoreError` (9 variants) has a stable `code()` → `&'static str`; `error_code_is_stable`
      pins every string
- [x] `testing` feature provides `FakeProcessRunner` (needle-matched scripted `run`/`spawn`,
      records calls), `FakeDownloader` (in-memory blobs + real SHA-256 verify), `FakeClock`
      (`set`/`advance`), `InMemoryFs` (`BTreeMap`-backed, atomic write, list, recursive remove)
- [x] Unit tests: coord round-trip + failures, error codes stable, every fake behaviour-tested,
      all 3 valid `.emuprofile` fixtures parse + `validate()`, invalid fixtures rejected
- [x] `cargo test -p emu-core --all-features` green — **39 tests**. Also green without the flag
      (self dev-dependency enables `testing` for the test build).

## Validate

```
cargo test -p emu-core --all-features
```

## Notes / findings

### Module layout (`crates/emu-core/src/`)

- `error.rs` — `CoreError` + `code()` + `parse()` / `invalid()` constructors.
- `model/` — `device`, `image`, `emulator`, `host`, `job`, `plan`, `profile`; `model/mod.rs`
  re-exports the public surface.
- `ports.rs` — `Command`/`Output` builders, `ChildProcess`, `ProcessRunner`, `Downloader`
  (`fetch(url, into, expected_sha256, job)`), `HostProbe`, `Clock`, `Fs`, plus `pct_progress`.
- `provider.rs` — `Provider` trait, `LaunchOpts`, `RunningHandle`.
- `testing/mod.rs` — the four fakes (feature `testing`).

### Decisions / deviations

- **`specta::Type` → task 0004** (see acceptance note). When adding it, also add
  `#[derive(specta::Type)]` to the DTOs in `ports.rs` / `provider.rs` that cross IPC
  (`LaunchOpts`, `RunningHandle`, `ImageFilter`, `CreateSpec`, `Plan`, `HostReport`, `Job`, …).
- **serde tags are explicit, not `rename_all`**: `ImageType` / `Abi` use per-variant
  `#[serde(rename = "...")]` equal to `.tag()` so the wire form and the `sdkmanager` CLI form
  can never diverge. Internally-tagged enums (`EmulatorSource`, `Verdict`, `RequirementStatus`)
  carry `rename_all_fields = "camelCase"` (needs serde ≥ 1.0.157).
- **`EmuProfile` mirrors `schemas/emuprofile/v1.schema.json`, not the domain types.** It has its
  own `ProfileImageType` / `ProfileGraphics` with `From` conversions to `ImageType` / `Graphics`.
  Two schema/CLI naming mismatches are documented in `profile.rs` for M4 to reconcile:
  `android-automotive_playstore` (schema) vs `android-automotive-playstore` (sdkmanager tag);
  `graphics: "hardware"` (schema) vs `-gpu host` (emulator).
- **`emu-core` is more permissive than the JSON Schema by design** — e.g. it accepts
  `armeabi-v7a` in an `Abi`; the schema is the gate that rejects it at import. So the
  `invalid/bad-abi.json` fixture parses fine at the serde layer (no test asserts otherwise).
- New deps (all non-`tauri`): `serde`, `async-trait`, `time` (`serde-well-known`, `macros`),
  `url`, `sha2`. Dev-only: `serde_json`, `tokio` (`rt`, `macros`), and a self dev-dependency
  with `features = ["testing"]` so `cargo test -p emu-core` works without `--all-features`.
- `clippy.toml` `doc-valid-idents` gained `MiB`/`GiB`/`KiB`/`TiB`.

### Not done here (by scope)

- No `crates/emu-core/tests/` integration file — unit tests inline next to each module cover
  the acceptance list; add crate-level integration tests when there's cross-module behaviour
  to exercise (M1+).
- `JobHandle` uses a plain boxed callback, not an async channel — keeps `emu-core` free of an
  async-runtime dep. The orchestrator (M1) wraps a real event sender.
