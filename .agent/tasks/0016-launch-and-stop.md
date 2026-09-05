---
id: "0016"
title: "emu-android: AndroidProvider — launch (spawn + boot-poll) + stop"
milestone: "M2"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

`launch` spawns `emulator @<avd_name>` via `ProcessRunner::spawn`, streams its stdout/stderr lines
onto the job's log, polls `adb wait-for-device` + `adb shell getprop sys.boot_completed` until it
reads `1`, then captures the adb serial + emulator pid + gRPC port into a `RunningHandle`. `stop`
sends `adb -s <serial> emu kill` (falling back to process kill) and waits for exit.

## Context / links

- Architecture: `docs/architecture.md` §5 ("Create + launch" steps 5-6)
- `crates/emu-core/src/provider.rs` (`Provider::launch`/`stop`, `RunningHandle`, `LaunchOpts`)
- `crates/emu-core/src/ports.rs` (`ChildProcess`, `ProcessRunner`)
- Real accelerator/graphics flags (`-accel`, `-gpu`) must be cited from `emulator -help` output,
  not invented (`AGENTS.md` §6 rule 2)

## Scope — files this task may touch

- `crates/emu-android/src/provider.rs` (extend `AndroidProvider` with `launch`/`stop`)
- Fixtures: captured `emulator -help`/boot log lines, `adb devices` / `getprop` output

## Acceptance criteria

- [ ] `launch` builds real argv (accelerator/graphics flags appropriate to the host, headless vs.
      windowed from `LaunchOpts`), spawns, streams log lines onto `JobHandle`
- [ ] Boot-complete polling has a sane timeout and a clear timeout error (not a hang)
- [ ] `stop` is graceful first, force-kill as a documented fallback; never orphans the process
- [ ] Unit tests with a fake `ProcessRunner`/`ChildProcess` simulating boot-complete lines and a
      timeout case
- [ ] `just check-fast` passes

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just check-fast
```

## Notes / findings

(Not started.)
