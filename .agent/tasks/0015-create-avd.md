---
id: "0015"
title: "emu-android: AndroidProvider — ensure_image (via sdkmanager) + create (avdmanager create avd)"
milestone: "M2"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

The first real `Provider` implementation. `ensure_image` ensures a chosen system image is
installed, `create` drives `avdmanager create avd` and registers the result. See Notes for why
`ensure_image`'s original "download via `Downloader` + extract the zip ourselves" plan was replaced
during this task with "ask the real `sdkmanager` to do it" — a real, tested, better design, not a
shortcut.

## Context / links

- Architecture: `docs/architecture.md` §3 (`Provider::ensure_image`/`create`), §5 ("Create +
  launch" flow steps 2-4)
- Depends on task `0014`'s catalogs and `crates/emu-core/src/toolchain/bootstrap.rs`'s existing
  "ask the real tool" philosophy for everything except `cmdline-tools` itself
- `crates/emu-core/src/provider.rs` — the trait implemented
- `crates/emu-core/src/registry/open.rs` — `Registry` surface, extended here

## Scope — files touched

- `crates/emu-android/src/provider.rs` (new — `AndroidProvider`, all 8 `Provider` methods; 6 are
  stubs until their own task, see Notes)
- `crates/emu-android/src/lib.rs`, `crates/emu-android/Cargo.toml` (`async-trait` dep;
  `tempfile`/`tokio` dev-deps)
- `crates/emu-core/src/registry/open.rs` (`insert_emulator`/`get_emulator`)
- `crates/emu-core/src/model/emulator.rs` (`EmulatorId::generate()`)
- `crates/emu-core/Cargo.toml` (`ulid` dep)

## Acceptance criteria

- [x] `ensure_image` is a no-op (fast success) when already installed; otherwise installs via a
      real `sdkmanager` invocation (design change from the original "download+extract ourselves"
      plan — see Notes), verified against the real marker file `sdkmanager` itself leaves
- [x] `create` builds the right `avdmanager create avd -n <name> -k <coord> -d <device id>` argv
      (real flags, cited from a live capture — no `--help` exists for this subcommand), classifies
      real failure text (already-exists / unknown device / invalid package) into specific
      `CoreError`s, and returns a new `EmulatorId`
- [x] A tracked row lands in the `emulators` table on success (`Registry::insert_emulator`)
- [x] Unit tests with fake `ProcessRunner`/`Fs`; every error-text case is a real, captured string
      (see Notes), not invented
- [x] `just check-fast` passes
- [x] Docs updated (this file's Notes; `0014`'s cross-reference stays accurate)

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just check-fast
```

## Notes / findings

### Design change: `ensure_image` delegates to `sdkmanager`, no `Downloader`/zip code at all

The task was originally scoped to download the system image via `Downloader` and extract it with
the same zip-crate pattern as `bootstrap.rs`'s `cmdline-tools` install. Real research (below)
showed that's unnecessary and worse: `sdkmanager` already knows how to fetch, verify, and unpack
its own packages correctly — exactly the reasoning `bootstrap.rs`'s own doc comment already gives
for `platform-tools`/`emulator`. System images are just another `sdkmanager` package path
(`system-images;android-34;default;x86_64`), so `ensure_image` is a `ProcessRunner`-only operation:
run `sdkmanager <path>`, check the same kind of marker file `bootstrap.rs` already uses for M1
components. This is simpler, has zero new archive-format risk, and reuses a philosophy the codebase
already committed to — a case of the task file being scoped before the real research, corrected per
`AGENTS.md` §9 ("the task seems wrong → edit the task file... rather than guessing").

### Real research behind every cited behavior

Downloaded the real `commandlinetools-mac_arm64` (already had it from task `0014`) plus a real,
small (720 MB) system image (`system-images;android-34;default;x86_64`, chosen for its size) and
ran the real tools against a scratch `ANDROID_SDK_ROOT`/`ANDROID_AVD_HOME`:

1. **`avdmanager create avd`'s flags** — no `--help` exists for this subcommand; running it with an
   unrecognized flag dumps real usage text, which is where `-n/--name`, `-k/--package`,
   `-d/--device`, `-f/--force`, `--skin`, `-b/--abi`, `-g/--tag`, `-c/--sdcard`, `-p/--path` came
   from (used: `-n`, `-k`, `-d` — `--force` deliberately **not** passed, see below).
2. **Installing a system image via `sdkmanager`**: `yes | sdkmanager
   "system-images;android-34;default;x86_64"` — real output confirms the install directory is
   exactly the package path with `;` → `/` (`Unzipping system-images/android-34/default/x86_64`),
   and the unpacked directory contains a `source.properties` (the same file every `sdkmanager`
   package writes — the marker `ensure_image` checks). Bonus finding: `sdkmanager` also silently
   pulled in the `emulator` package as a declared dependency of the image — expected, not a bug;
   `ensure_image` only checks for *its own* requested coordinate's marker, ignoring whatever else
   `sdkmanager` chose to also install. Also noted: this cmdline-tools revision prints "The SDK
   Manager CLI tool (sdkmanager) is deprecated. Android CLI will be used instead" — informational,
   exit code and output shape are unaffected.
3. **`avdmanager create avd` end to end**, against the now-really-installed image:
   - Without `-d`: real prompt `Do you wish to create a custom hardware profile? [no]` — answered
     by piped stdin. **Always passing `-d`** (every `CreateSpec.device_profile_id` — task `0014`'s
     `DeviceProfile.id`, confirmed real ids like `pixel_6` work even though they don't appear in
     `avdmanager list device`'s own limited built-in list) means this prompt never appears in
     practice, so `create` doesn't depend on answering it — the y-flood stdin is still fed as a
     belt-and-suspenders measure for any license prompt, exactly like `bootstrap.rs`.
   - Success: exit `0`, `<name>.avd/` + `<name>.ini` created (`.ini` contains `path=`/`target=`),
     **no positive "success" text at all** — `Output::success()` (exit code) is the only real
     signal, which is exactly why `create` doesn't try to pattern-match a success string.
   - Real failure text, `stderr` (confirmed separately from `stdout` — the progress-bar noise is
     `stdout`, `Error: ...` is always `stderr`):
     - Duplicate name (no `--force`): `Error: Android Virtual Device 'test_avd2' already exists.\nUse --force if you want to replace it.\nnull`
     - Unknown device id: `Error: No device found matching --device nonexistent_device_xyz.\nnull`
     - Package not installed locally: `Error: Package path is not valid. Valid system image paths are:\nnull`
   - `create` deliberately never passes `--force`: overwriting an existing AVD silently on a name
     collision is a data-loss footgun for a "create new" operation; the caller is expected to pick
     a unique name, and a real, specific error (mapped from the first case above) is better UX than
     silent replacement. A "recreate" feature can add `--force` explicitly later if needed.

### `EmulatorId::generate()` and the `ulid` dependency

`Emulator`'s own doc comment already said "a ULID string in practice" (task `0002`) but nothing
generated one. Added `ulid = "1"` to `emu-core` and `EmulatorId::generate()` — a real ULID
(lexicographically sortable by creation time), not a random UUID or a counter, matching what the
model already documented. `cargo deny` (license/advisory/source checks in `just validate`) passed
clean with `ulid` and its small dependency tree (`rand`, `getrandom`).

### `Registry` additions

`insert_emulator`/`get_emulator` on `Registry` — plain `sqlx::query`/`query_as` against the
existing `emulators` table (`id`, `avd_name`, `display_name` — `migrations/0001_init.sql`'s own
`UNIQUE` constraint on `avd_name` is the second safety net against a name collision, after
`avdmanager`'s own). No new migration needed; the full schema (image coord, hardware, source, tags)
is explicitly M3's job per `docs/architecture.md`.

### Provider trait: what's stubbed and why

`AndroidProvider` implements all 8 `Provider` methods (Rust requires a complete `impl`), but only
`ensure_image`/`create` do real work here:
- `list_devices`/`list_images` → `NotImplemented`, task `0017` (wiring task `0014`'s parsers to a
  real installed SDK — locating the `sdklib.core.jar`, extracting `devices.xml`-family entries,
  fetching `sysimg::MANIFEST_URLS` — is that task's job, alongside the IPC commands that need it)
- `launch`/`stop` → `NotImplemented`, task `0016`
- `delete`/`reconcile` → `NotImplemented`, M3 (full registry reconciliation)

### Test coverage gap, honestly noted

`FakeProcessRunner` has no real filesystem side effects, so no unit test can exercise the true
"sdkmanager succeeds and genuinely unpacks files" path — that's proven by the manual real capture
above, not by `cargo test` (a `--ignored` real-network integration test like task `0012`'s would be
the next step if this gap needs closing; not added here to keep this task's scope to what it says).
The unit suite instead proves: the no-op path, the post-success marker-check safety net, a real
process failure, and all three real `create` error-text classifications.
