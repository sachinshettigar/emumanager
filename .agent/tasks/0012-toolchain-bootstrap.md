---
id: "0012"
title: "emu-core: toolchain manager — InstalledState scan + bootstrap sdkmanager"
milestone: "M1"
status: "done"
owner: "Claude Code"
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

- [x] `InstalledState` scans the data dir's `sdk/` subtree (via `Fs`) and reports which of the
      three M1 components are present, by checking for their known marker file/dir (e.g.
      `cmdline-tools/latest/bin/sdkmanager`, `platform-tools/adb`(`.exe`), `emulator/emulator`)
      — filesystem is the source of truth, nothing cached (matches the `installed` field pattern
      already used on `SystemImage`)
- [x] **System-SDK detection.** Before concluding a component is missing, also check: (1)
      `ANDROID_HOME` / `ANDROID_SDK_ROOT` env vars, (2) the OS-conventional Android Studio SDK
      path (`~/Library/Android/sdk` macOS, `~/Android/Sdk` Linux, `%LOCALAPPDATA%\Android\Sdk`
      Windows — cite Android Studio's own docs for these). Same marker-file check as above,
      against each candidate root. A component found in a system root is reported `installed`
      with *where* it was found (app-managed vs. system path) so later `create`/`launch` code
      knows which `sdkmanager`/`adb`/`emulator` binary to invoke — don't just return a bool and
      lose the location. Env-var/path lookup takes a plain injected function
      (`Fn(&str) -> Option<String>` or similar), not a new port trait — keeps this unit-testable
      without expanding `ports.rs` for a one-task need
- [x] `bootstrap()` only downloads/installs a component that is missing from **both** the
      app-managed dir and every detected system root — a component satisfied by a system
      install is skipped entirely (no copy, no re-download)
- [x] `bootstrap(data_dir, components, ports) -> Result<()>`: downloads + unzips
      `cmdline-tools;latest` if missing, runs `sdkmanager --licenses` (feeds `y\n` × N to stdin —
      cite the real license-count/prompt behavior from a captured `--help`/session, don't guess),
      then `sdkmanager "platform-tools" "emulator"`
- [x] Decision recorded (see task 0011): where SHA verification happens given the SHA-1/SHA-256
      mismatch between the catalog and the `Downloader` trait
- [x] **Bundled JRE decision recorded, not silently skipped.** `docs/spec.md` §5.1 lists "bundled
      JRE" as detected/managed state. Modern `cmdline-tools` (26+) needs a system JDK 17+ on
      `PATH`/`JAVA_HOME` — Google no longer ships one. Either: (a) v1 requires a system JDK and
      `InstalledState`/`bootstrap` surface a clear "no JDK found, install one and set JAVA_HOME"
      error instead of silently failing inside `sdkmanager`, with real bundling deferred to a
      follow-up task/ADR (recommended — matches "keep it simple"), or (b) bundle a portable JRE
      now (e.g. Eclipse Temurin, cited source). Pick one, write it down here, and if (a), add the
      clear-error acceptance criterion instead of a download one.
- [x] Unit tests with `FakeProcessRunner`/`FakeDownloader`/`InMemoryFs` cover: fresh bootstrap,
      already-bootstrapped no-op, a failed download surfaces the real error, license-accept
      stdin content is exactly what was captured
- [x] `#[ignore]`d integration test: real `bootstrap()` against a scratch temp dir with the real
      ports from task 0011; asserts `sdkmanager --version` (via a real `ProcessRunner::run`)
      succeeds afterward
- [x] `just check-fast` passes; `just test-integration` runs the new ignored test when invoked
      explicitly (still excluded from `just validate`)

## Validate

```
cargo test -p emu-core --all-features
just test-integration
just check-fast
```

## Notes / findings

### License-prompt behavior — captured live, not guessed

Downloaded the real `cmdline-tools` (`commandlinetools-mac-13114758_latest.zip`, cmdline-tools
`19.0`) and ran `yes | sdkmanager --licenses` for real against a scratch `ANDROID_SDK_ROOT`, JDK 21
on `PATH`, 2026-09-05. Real transcript shape:

```text
7 of 7 SDK package licenses not accepted.
1/7: License android-googletv-license:
<full license text>
Accept? (y/N): 
2/7: License android-googlexr-license:
...
7/7: License mips-android-sysimage-license:
...
Accept? (y/N): All SDK package licenses accepted
```

Each license prints its full text then a literal `Accept? (y/N):` prompt reading one line of
stdin; after the last one it prints `All SDK package licenses accepted`. The count (7 here) is
**not stable** — it's whatever licenses exist in the current repository catalog, which changes
over time — so `bootstrap()` doesn't hardcode 7. It feeds `LICENSE_ACCEPT_COUNT = 50` `y\n`
answers up front and closes stdin: comfortably more than any real count in the SDK's history,
still finite (unlike piping `yes` forever), and the extra unread `y`s are simply never consumed
once `sdkmanager` exits. Also captured: `sdkmanager --version` prints a bare version (`19.0`) to
stdout and exits `0`; accepted licenses land as one file per license id under `<sdk>/licenses/`
(no extension) — not used directly by this task, but useful context for anyone touching this area
later.

### SHA-1 vs SHA-256 — verified at the call site, trait unchanged

`fetch_and_extract_cmdline_tools` calls `Downloader::fetch` with `expected_sha256: None` (the
catalog, task 0010, only has SHA-1) and instead reads the downloaded bytes back via `Fs::read` and
hashes them with the `sha1` crate, comparing against `Component.sha1` itself. `ports.rs`'s
`Downloader` trait is untouched — task 0011's Notes flagged this as the deferred decision, and
changing the trait signature for one caller wasn't warranted.

### Bundled JRE — decided: require a system JDK 17+ (option a)

See `docs/adr/0006-require-system-jdk.md` (new) and `docs/spec.md` §8's open question, updated
to point at it. `bootstrap()` calls `java -version` before anything else and fails fast with a
specific, actionable error if no JDK 17+ is found or reachable — verified for real against the
actual JDK 21 on this machine (`java -version` prints its version line to **stderr**, not stdout;
the code checks whichever stream is non-empty). Parses both the pre-JDK9 (`"1.8.0_372"`) and JEP
223 (`"17.0.9"`, `"21"`) version-string shapes.

### Only `cmdline-tools` is downloaded directly; `sdkmanager` installs the rest

`platform-tools` and `emulator` are **not** fetched via `Downloader`/the catalog URL by this
module — once `cmdline-tools` exists, `sdkmanager "platform-tools" "emulator"` (one real
subprocess call, package paths from `ComponentId::repo_path()`) does the job exactly the way a
human would, using its own real resolver/downloader/checksum logic. Re-implementing that for two
more components would have been needless surface area for zero benefit. Only components still
`missing` after excluding `cmdline-tools` are passed to this call — one already satisfied by a
system SDK is skipped, per the reuse requirement.

### Archive extraction — through `Fs`, not a bypass, permission bit fixed up via `chmod`

`extract_cmdline_tools` unpacks entirely through the `Fs` port (`ensure_dir`/`write_atomic`), so
it's exercisable with `InMemoryFs` in the fake-driven unit tests — no bypass to real `std::fs`.
`Fs::write_atomic` doesn't carry a permission mode, so the extracted files land without the
executable bit; `make_bin_executable` fixes this with one `chmod -R +x <bin dir>` call through the
existing `ProcessRunner` port (a no-op on Windows, where `chmod` doesn't exist and executability
isn't permission-bit-based). This avoids both inventing a fifth port trait and touching
`ports.rs`/`NativeFs` for a single self-contained operation — same philosophy the task itself used
for system-SDK env lookup.

Only `cmdline-tools`'s zip needs special handling: its real archive (verified by actually
downloading and inspecting it, 2026-09-05) has a top-level `cmdline-tools/` folder that must be
*renamed* to `latest/` — that placement is an Android convention the zip itself doesn't encode, not
something `unzip -d` or `tar --strip-components` can express against the abstract `Fs` trait, so
the leading path segment is stripped and replaced manually while walking the archive with the
`zip` crate (added as a real, pure-Rust dependency — no reliance on a system `unzip`/`tar` binary,
which would itself be a small "external setup" gap against `docs/spec.md` goal 1).

### New public API: `toolchain::binary_path`

`ComponentLocation` carries the SDK root a component was found under specifically so later code
knows which binary to invoke — `installed_state::binary_path(sdk_root, id, os)` is the public
helper that turns that into an actual path (`cmdline-tools/latest/bin/sdkmanager(.bat)`,
`platform-tools/adb(.exe)`, `emulator/emulator(.exe)`). Used by `bootstrap` itself and by the
integration test; M2's `create`/`launch` code should use the same helper rather than re-deriving
these paths.

### Integration test can't reuse `src-tauri`'s real ports directly

`emu-core` must never depend on `tauri` (AGENTS.md §6.1), and `src-tauri` depends on `emu-core`
(not the reverse), so `crates/emu-core/tests/toolchain_bootstrap.rs` can't import task 0011's
`NativeFs`/`NativeDownloader`/`NativeProcessRunner`. It defines small, test-local stand-ins over
the same real `tokio::fs`/`reqwest`/`tokio::process` instead — real enough to prove the real path
end to end, without production's atomic-write/progress-throttling niceties this one-shot test
doesn't need. It also dev-depends on `emu-android` (a supported Cargo dev-dependency cycle — affects
only test builds) to resolve today's *real* component catalog rather than hardcoding a URL.

**Actually run** (not just written): `cargo test -p emu-core --all-features --test
toolchain_bootstrap -- --ignored` passed for real on 2026-09-05 — downloaded the real
`cmdline-tools` and `platform-tools` from Google into a scratch temp dir, accepted licenses,
installed `platform-tools` via a real `sdkmanager`, and `sdkmanager --version` succeeded
afterward. ~35s with this machine's network.

### Not done here

- No JRE bundling (see the ADR) — a system JDK is required for v1.
- `emulator` is not exercised by the integration test (DoD only asks for `cmdline-tools` +
  `platform-tools`) — it's a much larger download; the same `sdkmanager` install path covers it
  identically to `platform-tools` and is exercised by the fake-driven unit tests' `wanted` sets in
  the general case, just not this real-network test.
- No download progress UI wiring or IPC command — that's task 0013.
