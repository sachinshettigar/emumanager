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

**Reuse what's already on the machine — never re-download a component that's already usable.**
`InstalledState` must check for an existing system Android SDK (`ANDROID_HOME`/
`ANDROID_SDK_ROOT`, or an Android Studio default install path per OS) *in addition to* the app's
own managed `sdk/` dir, and treat a component found there as installed too. `bootstrap()` only
downloads what's missing from *both* locations. See the new acceptance criterion below — this
was called out explicitly (not something to skip) because the naive version of this task (scan
only the app's own dir) would re-download a multi-GB SDK on every machine that already has
Android Studio, which is exactly the friction `docs/spec.md` goal 1 exists to remove.

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
- [ ] **System-SDK detection.** Before concluding a component is missing, also check: (1)
      `ANDROID_HOME` / `ANDROID_SDK_ROOT` env vars, (2) the OS-conventional Android Studio SDK
      path (`~/Library/Android/sdk` macOS, `~/Android/Sdk` Linux, `%LOCALAPPDATA%\Android\Sdk`
      Windows — cite Android Studio's own docs for these). Same marker-file check as above,
      against each candidate root. A component found in a system root is reported `installed`
      with *where* it was found (app-managed vs. system path) so later `create`/`launch` code
      knows which `sdkmanager`/`adb`/`emulator` binary to invoke — don't just return a bool and
      lose the location. Env-var/path lookup takes a plain injected function
      (`Fn(&str) -> Option<String>` or similar), not a new port trait — keeps this unit-testable
      without expanding `ports.rs` for a one-task need
- [ ] `bootstrap()` only downloads/installs a component that is missing from **both** the
      app-managed dir and every detected system root — a component satisfied by a system
      install is skipped entirely (no copy, no re-download)
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
