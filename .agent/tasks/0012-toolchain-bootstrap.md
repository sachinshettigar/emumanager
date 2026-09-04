---
id: "0012"
title: "emu-core: toolchain manager — InstalledState scan + bootstrap sdkmanager"
milestone: "M1"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

The orchestration that turns an empty data dir into a working, licensed `sdkmanager` with
`platform-tools` and `emulator` installed: `InstalledState` (what's on disk right now) and
`bootstrap()` (fetch + unzip `cmdline-tools`, run `sdkmanager --licenses` non-interactively, then
`sdkmanager` to install the rest). Pure orchestration in `emu-core` against the `ports.rs` traits
— fake-driven unit tests plus one real `#[ignore]`d integration test.

## Context / links

- Architecture: `docs/architecture.md` §3 ("Toolchain manager" bullet), §6 (integration tests are
  `--ignored`, not in `just validate`)
- Depends on: task 0010 (`Component` catalog), task 0011 (real ports to exercise in the
  integration test)
- MILESTONES.md M1 DoD: "on a clean CI job with an empty data dir, a `--ignored` integration test
  downloads `cmdline-tools` + `platform-tools` and `sdkmanager --version` succeeds"

## Scope — files this task may touch

- `crates/emu-core/src/toolchain/mod.rs`, `.../installed_state.rs`, `.../bootstrap.rs` (new)
- `crates/emu-core/src/lib.rs` (module wiring)
- `crates/emu-core/tests/toolchain_bootstrap.rs` (new — the `#[ignore]`d real-network test)
- `justfile` (`test-integration` recipe, if it doesn't already run crate-level `--ignored` tests)

## Acceptance criteria

- [ ] `InstalledState` scans the data dir's `sdk/` subtree (via `Fs`) and reports which of the
      three M1 components are present, by checking for their known marker file/dir (e.g.
      `cmdline-tools/latest/bin/sdkmanager`, `platform-tools/adb`(`.exe`), `emulator/emulator`)
      — filesystem is the source of truth, nothing cached (matches the `installed` field pattern
      already used on `SystemImage`)
- [ ] `bootstrap(data_dir, components, ports) -> Result<()>`: downloads + unzips
      `cmdline-tools;latest` if missing, runs `sdkmanager --licenses` (feeds `y\n` × N to stdin —
      cite the real license-count/prompt behavior from a captured `--help`/session, don't guess),
      then `sdkmanager "platform-tools" "emulator"`
- [ ] Decision recorded (see task 0011): where SHA verification happens given the SHA-1/SHA-256
      mismatch between the catalog and the `Downloader` trait
- [ ] **Bundled JRE decision recorded, not silently skipped.** `docs/spec.md` §5.1 lists "bundled
      JRE" as detected/managed state. Modern `cmdline-tools` (26+) needs a system JDK 17+ on
      `PATH`/`JAVA_HOME` — Google no longer ships one. Either: (a) v1 requires a system JDK and
      `InstalledState`/`bootstrap` surface a clear "no JDK found, install one and set JAVA_HOME"
      error instead of silently failing inside `sdkmanager`, with real bundling deferred to a
      follow-up task/ADR (recommended — matches "keep it simple"), or (b) bundle a portable JRE
      now (e.g. Eclipse Temurin, cited source). Pick one, write it down here, and if (a), add the
      clear-error acceptance criterion instead of a download one.
- [ ] Unit tests with `FakeProcessRunner`/`FakeDownloader`/`InMemoryFs` cover: fresh bootstrap,
      already-bootstrapped no-op, a failed download surfaces the real error, license-accept
      stdin content is exactly what was captured
- [ ] `#[ignore]`d integration test: real `bootstrap()` against a scratch temp dir with the real
      ports from task 0011; asserts `sdkmanager --version` (via a real `ProcessRunner::run`)
      succeeds afterward
- [ ] `just check-fast` passes; `just test-integration` runs the new ignored test when invoked
      explicitly (still excluded from `just validate`)

## Validate

```
cargo test -p emu-core --all-features
just test-integration
just check-fast
```

## Notes / findings

(License-prompt behavior must be captured from a real `sdkmanager --licenses` session output —
cite the SDK cmdline-tools version used. JDK decision goes here.)
