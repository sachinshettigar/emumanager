---
id: "0002"
title: "emu-core ports and models (no impls)"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
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

- [ ] Model structs match `docs/context/domain-model.md`; all derive
      `serde::{Serialize,Deserialize}` and `specta::Type`
- [ ] `ImageCoord`, `ImageType`, `Abi` with `Display`/`FromStr` producing/parsing
      `system-images;android-34;google_apis_playstore;x86_64`
- [ ] Port traits are `async` (via `async-trait` or native), object-safe where used as `dyn`
- [ ] `CoreError` has a stable `code()` -> `&'static str` for the IPC layer
- [ ] `testing` feature provides `FakeProcessRunner` (scripted stdout/exit), `FakeDownloader`
      (in-memory), `FakeClock`, `InMemoryFs`
- [ ] Unit tests: coord round-trip (incl. failure cases), error codes stable,
      fakes behave as specified
- [ ] `cargo test -p emu-core --all-features` green

## Validate

```
cargo test -p emu-core --all-features
```

## Notes / findings
