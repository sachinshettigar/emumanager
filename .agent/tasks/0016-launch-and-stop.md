---
id: "0016"
title: "emu-android: AndroidProvider — launch (spawn + boot-poll) + stop"
milestone: "M2"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

`launch` spawns `emulator @<avd_name>` via `ProcessRunner::spawn`, streams its output onto the
job's log, polls `adb` until `sys.boot_completed=1`, and returns a `RunningHandle`. `stop` sends
`adb -s <serial> emu kill`, then force-kills the child if the emulator doesn't exit.

## Context / links

- Architecture: `docs/architecture.md` §5 ("Create + launch" steps 5-6)
- `crates/emu-core/src/provider.rs` (`Provider::launch`/`stop`, `RunningHandle`, `LaunchOpts`)
- `crates/emu-core/src/ports.rs` (`ChildProcess`, `ProcessRunner`)
- Emulator flags cited from the official reference:
  <https://developer.android.com/studio/run/emulator-commandline>

## Scope — files touched

- `crates/emu-android/src/provider.rs` (`launch`/`stop` + helpers; `running` child-handle map;
  `with_boot_timeout`/`with_stop_timeout` builders)
- `crates/emu-android/Cargo.toml` (`tokio` `time`+`sync` features)
- `crates/emu-core/src/testing/mod.rs` (`FakeProcessRunner::on_spawn_lingering` + `FakeChild`
  `lingering` mode — see Notes)

## Acceptance criteria

- [x] `launch` builds real argv from `LaunchOpts` (`-no-window`, `-wipe-data`,
      `-no-snapshot-load`, `-gpu <mode>`, plus `extra_args`), spawns, streams log lines onto the
      `JobHandle`
- [x] Boot-complete polling has a sane timeout ([`DEFAULT_BOOT_TIMEOUT`] = 300 s, overridable) and
      a clear timeout error — plus a fast-fail if the emulator process exits before boot
- [x] `stop` is graceful (`adb emu kill`) first, force-kills the held child process as the
      documented fallback, and reaps the handle either way; idempotent when nothing is running
- [x] Unit tests: boot-completes, streams-then-boots, exits-before-boot (fast fail), never-boots
      (timeout), `emu kill` to the AVD-matched serial, idempotent stop, unknown-id, force-kill
      fallback
- [x] `just check-fast` passes

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just check-fast
```

## Notes / findings

### Emulator flags — cited, not invented

From the official emulator command-line reference (checked 2026-09-05): launch by name is
`emulator @<avd_name>`; `-no-window` = headless; `-gpu <mode>` with `auto` / `host` /
`swiftshader_indirect` (the three `Graphics` variants); `-no-snapshot-load` = cold boot;
`-wipe-data` = wipe user data. `launch` passes only what `LaunchOpts` asks for plus `extra_args`
— no `-accel` flag (the emulator's own `auto` default handles acceleration; host-readiness checks
are M5's `emu-host`), no `-no-audio` (a windowed dev emulator may want audio; nothing here should
silently strip it).

### `adb` bits — cited

- `adb devices` output: header line `List of devices attached`, then `emulator-NNNN\t<state>` per
  device. `emulator_serials` skips the header and takes every `emulator-*`.
- Boot check: `adb -s <serial> shell getprop sys.boot_completed` prints `1` once booted. An adb
  error (device still `offline`) counts as "not booted, keep polling".
- `stop`'s serial lookup: for each `emulator-*`, `adb -s <serial> emu avd name` (an emulator
  console command) replies with the AVD name on its own line then `OK` — matched against the
  registry's `avd_name` so `stop` never kills the wrong emulator when several are running.
- Graceful shutdown: `adb -s <serial> emu kill`.

### `launch` design decisions

- **Holds the spawned child** in `AndroidProvider.running` (an `EmulatorId → Arc<Mutex<Box<dyn
  ChildProcess>>>` map) rather than dropping it after boot. That's what lets `stop` reap the
  process ("never orphans" in the acceptance criteria) and keeps the child's stdout pipe drained
  (via the `NativeProcessRunner` line forwarders) so a chatty emulator never blocks on a full
  buffer.
- **`grpc_port` is left `None`.** Parsing it out of the emulator's log, plus a continuous
  post-boot log stream, are the "Live console" / detail-panel work in `docs/spec.md` §5.3 → M3.
- **Timeouts are wall-clock**, checked against a deadline; `DEFAULT_BOOT_TIMEOUT` 300 s,
  `DEFAULT_STOP_TIMEOUT` 15 s, both overridable via builder methods (used by the tests to run in
  milliseconds).

### Testing-infra addition: `FakeChild` lingering mode

A real emulator's output stream stays open for the life of the process; `FakeChild` previously
always hit EOF after its scripted lines, so the pure "stream still open, boot never completes,
wall-clock deadline fires" path couldn't be tested (EOF would trigger the fast-fail branch first).
Added `FakeProcessRunner::on_spawn_lingering` + a `lingering` flag on `FakeChild`: after the
scripted lines, `next_line()` never resolves (until `kill()`), so a caller polling with a
`timeout` sees `Elapsed`, not EOF. This is a general improvement to the fake, not a one-off — it
models any long-lived child. Used by `launch_times_out_when_boot_never_completes` and
`stop_force_kills_when_graceful_shutdown_is_ignored`.

### Honest gap

No unit test exercises a *real* `emulator`/`adb` process (no network / no Android binaries in
`just validate`, per `AGENTS.md` §6.4). The argv construction, the poll loop, the timeout branch,
and the stop fallbacks are all covered by fake-driven tests; end-to-end boot is the `tauri-driver`
E2E in M2's DoD (task `0017`) and/or a future `--ignored` integration test (task `0012`'s pattern).
On Unix, a launched emulator whose `stop` never runs (app killed) leaves the child unreaped —
proper kill-safety/reconciliation is M3's explicit milestone (`docs/spec.md` §5.3, MILESTONES M3).
