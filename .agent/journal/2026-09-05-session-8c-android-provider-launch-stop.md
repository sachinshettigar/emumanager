# 2026-09-05 — session 8 (continued) (Claude Code)

## Worked on

- Task `0016` (done): `emu-android::AndroidProvider::launch` + `stop` — the provider's runtime
  surface. M2 is now one task from complete (`0017` left).

## Changed

- `crates/emu-android/src/provider.rs` — `launch`/`stop` + helpers (`emulator_serials`,
  `boot_completed`, `serial_for_avd`, `avd_still_running`, `wait_for_boot`, `gpu_mode`); a
  `running: Mutex<HashMap<EmulatorId, Arc<AsyncMutex<Box<dyn ChildProcess>>>>>` field holding
  spawned children; `with_boot_timeout` / `with_stop_timeout` builders.
- `crates/emu-android/Cargo.toml` — `tokio` `time` + `sync` features (no `rt` — runs on the
  caller's runtime).
- `crates/emu-core/src/testing/mod.rs` — `FakeProcessRunner::on_spawn_lingering` + a `lingering`
  flag on `FakeChild` (`next_line` never resolves after the scripted lines, until `kill`).

## State now

- `just validate`: **pass** (109 Rust tests; no IPC surface touched — `bindings.ts` no-op diff)
- `just progress`: pass
- `cargo test -p emu-core -p emu-android --all-features`: 56 + 36 passed, 0 failed (8 new)
- `lastValidatedCommit`: set to this session's commit after pushing

## Next action

Task `0017` — the last M2 task. `tauri-specta` commands + a generalized job event in
`src-tauri/src/commands/emulator.rs`; wire `src/routes/Create.tsx` (device picker → image picker
with inline install → hardware → review) and `src/routes/Dashboard.tsx` (list tracked emulators
with live Stopped/Booting/Running + stop) to the real `AndroidProvider`. This task also has to
wire `AndroidProvider::list_devices`/`list_images` for real — locate the installed
`sdklib.core.jar`, extract the `com/android/sdklib/devices/*.xml` entries
(`devices::DEVICE_XML_RESOURCES`), fetch `sysimg::MANIFEST_URLS` through a `Downloader`, then feed
task `0014`'s parsers. Reading a jar = a zip; the `zip` crate is already an `emu-core` dep, but
this jar-reading belongs in `emu-android` (or `src-tauri`) — decide where when picking the task up.

## Gotchas / notes for the next agent

- **Emulator flags are cited from the official command-line reference**
  (<https://developer.android.com/studio/run/emulator-commandline>), not `emulator -help` output
  (no Android binaries in this environment). If task `0017` or later adds flags (skins, network
  speed, snapshots), re-check the same doc.
- **`AndroidProvider` holds spawned emulator children in `self.running`** — dropping that handle
  is what would orphan the process on Unix and let its stdout pipe fill. `stop` removes + reaps.
  If you add a code path that spawns an emulator, route it through the same map.
- **`launch`'s boot poll is wall-clock, not iteration-count** — `with_boot_timeout` /
  `with_stop_timeout` exist so tests can run in milliseconds. `POLL_INTERVAL` (2 s) and
  `LOG_DRAIN_SLICE` (250 ms) are plain consts; a test that needs the loop to actually iterate uses
  a `with_*_timeout` a bit larger than `LOG_DRAIN_SLICE`.
- **`FakeChild` lingering mode**: `on_spawn_lingering` for a process that stays running; plain
  `on_spawn` still means "emits lines then EOFs then exits". Don't use paused-time
  (`#[tokio::test(start_paused = true)]`) with `Registry::open` in the same test — sqlx's pool
  acquire times out against the virtual clock; use real time + tiny `with_*_timeout` instead.
- **`grpc_port` is `None`** from `launch` — wiring it (and a continuous log stream) is M3.
- CI still blocked on GitHub billing — nothing to do until a human fixes it.
