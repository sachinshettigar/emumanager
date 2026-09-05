---
id: "0019"
title: "AndroidProvider::reconcile + delete + kill-safety"
milestone: "M3"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Make the registry converge to ground truth. `AndroidProvider::reconcile()` reads the real AVD list
(`avdmanager list avd`), the running set (`adb devices` + `emu avd name`), and each row's stored
run-fields, then: adopts on-disk AVDs with no row, flags rows whose AVD vanished, refreshes
`last_state` / `adb_serial` / `grpc_port`, and — kill-safety — resets any row left `Booting`/`Running`
with a now-dead `pid` back to `Stopped`. `AndroidProvider::delete()` gets its real implementation.

## Context / links

- Milestones: `MILESTONES.md` M3 bullets 2, 4 (delete), 6 (kill-safety), and the DoD (property test)
- Architecture: `docs/architecture.md#5-main-flows`, `Provider` trait `reconcile` / `delete`
- Depends on task `0018` (typed `EmulatorRow`, `set_run_fields`, `delete_row`)
- `crates/emu-android/src/provider.rs` — `tracked_states` already does the "row + live adb state" walk;
  `reconcile` is that plus adopt/drop/persist.

## Scope — files this task may touch

- `crates/emu-android/src/provider.rs` (`reconcile`, `delete`, helpers)
- `crates/emu-android/src/avd_list.rs` (new — `parse_avdmanager_list_avd`, pure)
- `crates/emu-android/tests/fixtures/avdmanager-list-avd.txt` (new — real captured output)
- `crates/emu-android/tests/` — reconcile scenario + property test
- `crates/emu-core/src/testing/` — only if the fakes need a small addition (note it)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] `parse_avdmanager_list_avd(&str) -> Vec<AvdEntry { name, device, path, target }>` against a
      **real captured** `avdmanager list avd` fixture (cite the capture in a module comment; if no
      Android SDK is installed on this machine, capture it during the task the same way `0014`/`0015`
      captured theirs, or fall back to the documented output shape and say so).
- [ ] `reconcile()` returns `Vec<LiveState>` and, as a side effect, makes the registry match reality:
  - [ ] an AVD on disk with no registry row → inserted as `EmulatorSource::Manual { discovered: true }`
  - [ ] a registry row whose `avd_name` is not in `avdmanager list avd` → `last_state = Error` and a
        marker (do **not** hard-delete — the user may want to see it; real delete is explicit)
  - [ ] for every surviving row: `last_state` / `adb_serial` / `grpc_port` refreshed from adb
  - [ ] kill-safety: a row with `last_state` `Booting`/`Running` and a `pid` that is not alive
        (`ProcessRunner` can't check liveness directly → treat "not in `adb devices` and pid set" as
        dead) → reset to `Stopped`, clear `adb_serial` / `pid`
- [ ] `delete(id, wipe)`: `avdmanager delete avd -n <avd_name>` (cite flag), then `delete_row(id)`;
      `wipe` additionally removes the AVD data dir via `Fs`. `NotFound` for an unknown id; deleting a
      running emulator errors with a clear "stop it first" message.
- [ ] Property/integration test (the M3 DoD): a fake-driven loop that randomly
      creates / launches / kills (drops the child) / adopts / removes AVDs, calls `reconcile()`, and
      asserts the registry's `last_state` for every row equals the fake's ground truth. Fake-driven,
      no real binaries, in `just validate`.
- [ ] Tests for each bullet above; `just check-fast` then `just validate` green
- [ ] Docs updated (`MILESTONES.md` M3 boxes, architecture reconcile note)

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just validate
```

## Notes / findings

(Fill in: real `avdmanager list avd` capture or the fallback; how "pid alive" is approximated
without a new port; anything the property test flushed out.)
</content>
