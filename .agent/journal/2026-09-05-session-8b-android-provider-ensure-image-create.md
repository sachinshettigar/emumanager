# 2026-09-05 — session 8 (continued) (Claude Code)

## Worked on

- Task `0015` (done): `emu-android::AndroidProvider` — the first real `Provider` implementation
  (`ensure_image` + `create`).

## Changed

- `crates/emu-android/src/provider.rs` (new) — `AndroidProvider`; full `Provider` impl (2 of 8
  methods do real work, the rest stub to their own task — see the file's module doc).
- `crates/emu-android/Cargo.toml` — `async-trait` dep; `emu-core` (testing feature)/`tokio`/
  `tempfile` dev-deps.
- `crates/emu-android/src/lib.rs` — wires the new module.
- `crates/emu-core/src/model/emulator.rs` — `EmulatorId::generate()` (real ULID).
- `crates/emu-core/Cargo.toml` — `ulid` dep.
- `crates/emu-core/src/registry/open.rs` — `insert_emulator`/`get_emulator`.
- `.agent/tasks/0015-create-avd.md` — rewritten (goal/scope/acceptance criteria changed to match
  the real design discovered while working it — see Gotchas), `done`, full Notes.
- `.agent/state.json`, `MILESTONES.md`, `PROGRESS.md` updated.

## State now

- `just validate`: **pass** (no IPC surface touched — `bindings.ts` regenerated as a no-op diff)
- `just progress`: pass
- `cargo test -p emu-core -p emu-android --all-features`: 56 + 27 = 83 passed, 0 failed (17 new
  since task 0014: 8 in `emu-core`, 9 in `emu-android`)
- `lastValidatedCommit`: set to this session's commit after pushing

## Next action

Task `0016`: `AndroidProvider::launch` (spawn `emulator @<avd_name>` via `ProcessRunner::spawn`,
stream log lines onto the job, poll `adb` for `sys.boot_completed`, capture serial/pid/gRPC port
into a `RunningHandle`) + `stop` (graceful `adb emu kill`, force-kill fallback). Real accelerator/
graphics flags must be cited from a real `emulator -help` capture, not invented — same discipline
as this task's `avdmanager create avd` flags.

## Gotchas / notes for the next agent

- **`ensure_image` does not use `Downloader` or any zip-extraction code** — it runs a real
  `sdkmanager <system-images;...>` and lets the tool fetch+verify+unpack itself, exactly like
  `bootstrap.rs` already does for `platform-tools`/`emulator`. If you're tempted to add download/
  extraction code for system images, don't — read task `0015`'s Notes first for why that was tried
  and rejected mid-task.
- **The install directory for any `sdkmanager` package is its own repo path with `;` → `/`**,
  relative to the SDK root — confirmed for system images (`image_dir` in `provider.rs`), already
  true for `cmdline-tools;latest` → `cmdline-tools/latest`. If a later task adds another package
  kind, this convention almost certainly still holds — verify against a real install before
  assuming otherwise.
- **`avdmanager create avd` has no `--help`** — real flags only come from a live "unrecognized
  flag" usage dump or the (unversioned, can drift) Android tools source. If flags are ever added
  (e.g. `--sdcard`, `--skin` for a hardware-editor feature), re-capture for real rather than
  guessing from memory.
- **`create` never passes `--force`** — a deliberate choice (data-loss footgun on a name
  collision). Don't add it without a real "recreate" user action requiring it explicitly.
- **`AndroidProvider`'s 6 unimplemented `Provider` methods return `CoreError::NotImplemented` with
  a message naming the task that completes them** — keep that pattern (message names the task) for
  any future stub, so a caller's error text is itself a pointer to what to build next.
- **Test-coverage gap, intentional and documented**: no unit test proves `sdkmanager` truly unpacks
  a real system image (the fakes have no filesystem side effects) — that's the manual real capture
  recorded in task `0015`'s Notes, not `cargo test`. A `--ignored` real integration test (task
  `0012`'s `toolchain_bootstrap.rs` pattern) would close this if it ever becomes a real risk.
- CI is still blocked on GitHub billing — see session-4's journal entry. Nothing to do there until
  a human fixes it.
