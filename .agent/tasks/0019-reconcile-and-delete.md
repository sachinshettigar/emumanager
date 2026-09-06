---
id: "0019"
title: "AndroidProvider::reconcile + delete + kill-safety"
milestone: "M3"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-06"
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

- [x] `parse_avdmanager_list_avd(&str) -> Vec<AvdEntry>` (`crates/emu-android/src/avd_list.rs`)
      against a **real captured** fixture — `tests/fixtures/avdmanager-list-avd.txt`, produced this
      task from a real `cmdline-tools 16111833` install (downloaded `commandlinetools-mac_arm64`,
      `sdkmanager` for `system-images;android-24;default;x86_64` + `emulator` + `platform-tools`,
      two real `avdmanager create avd` runs + one hand-seeded broken AVD, then `avdmanager list avd`).
      `AvdEntry { name, device, path, tag_abi, based_on, loadable, error }`.
- [x] `reconcile()` returns `Vec<LiveState>` and makes the registry match reality:
  - [x] a **loadable** AVD on disk with no registry row → adopted as
        `EmulatorSource::Manual { discovered: true }`, enriched from its `config.ini`
        (`image.sysdir.1` → `ImageCoord`, `hw.device.name`, `avd.ini.displayname`, `hw.ramSize`)
  - [x] a registry row whose `avd_name` is missing from `avdmanager list avd`, **or present but
        un-loadable** (missing system image) → `last_state = Error`; never hard-deleted
  - [x] every surviving row: `last_state` / `adb_serial` / `grpc_port` / `pid` refreshed from adb
  - [x] kill-safety: a `Booting`/`Running` row whose AVD is not in `adb devices` → `Stopped`,
        `pid` / `adb_serial` / `grpc_port` cleared (see Notes on the pid-liveness approximation)
- [x] `delete(id, wipe)`: `wipe = true` → `avdmanager delete avd -n <name>` (removes the whole
      `.avd` dir, userdata included) then `delete_row`; `wipe = false` → `delete_row` only (untrack).
      `NotFound` for an unknown id; running → `Invalid` ("stop it before deleting"). A
      "no Android Virtual Device named" stderr on the wipe path is treated as success (already gone).
- [x] M3 DoD test: `reconcile_converges_to_ground_truth_over_random_scenarios` — 48 xorshift64
      pseudo-random arrangements of loadable / broken / running AVDs over a 6-name pool, each with a
      fresh provider + registry; after one `reconcile()` every row's `last_state` equals ground
      truth and every loadable AVD is tracked. Fake-driven, no real binaries, in `just validate`.
- [x] 11 new tests (3 `avd_list`, 8 `provider`); `just check-fast` then `just validate` green
      (128 rust tests, 22 web).
- [x] Docs updated (`MILESTONES.md` M3 bullets 2 / 4-delete / 6 / DoD, architecture reconcile note)

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just validate
```

## Notes / findings

### Real `avdmanager list avd` capture

No Android SDK was installed on this machine (only a Homebrew `adb`). Captured a real one the way
`0014`/`0015` did: downloaded `commandlinetools-mac_arm64-16111833_latest.zip` into the scratchpad,
unpacked to `cmdline-tools/latest/`, `sdkmanager` for `system-images;android-24;default;x86_64`
(399 MiB — the smallest modern `default` x86_64 image), `emulator`, `platform-tools`, then two real
`avdmanager create avd` runs (`-d pixel_6`, `-d wearos_small_round`) plus one hand-seeded `.ini` +
`config.ini` pointing at an absent image. `avdmanager list avd` then printed all three.

Real quirks the capture surfaced, all encoded in `avd_list.rs`'s parser + module doc:
- Loadable entry = a block of `Name:` / `Device:` / `Path:` / `Target:` / `Based on: … Tag/ABI:` /
  `Sdcard:`, blocks separated by a line of exactly nine dashes.
- `Target:` prints **empty** even with `platforms;android-24` installed (verified — installed it and
  re-ran); the android-version text is on the indented `Based on:` continuation line.
- Broken AVDs come after `The following Android Virtual Devices could not be loaded:` and carry only
  `Name:` / `Path:` / `Error:`. `avdmanager list avd -c` (compact) **omits them entirely**, which is
  why `reconcile` parses the verbose form.
- `avdmanager delete avd`: success is exit 0 + `\nAVD '<name>' deleted.`; unknown name is exit 1 +
  `Error: There is no Android Virtual Device named '<name>'.\nnull`. It removes the whole `.avd`
  directory (both the `.ini` and the dir), so there is no "keep the AVD, wipe only userdata" here —
  that is task `0020`'s `wipe_emulator_data` (launch with `-wipe-data`).
- `avdmanager create avd` needs the `emulator` package installed or it fails with
  `Error: "emulator" package must be installed!` before touching the AVD.

Fixture normalizations (documented in the parser module doc, neither touching parsed fields): the
machine-specific `Path:` prefix rewritten to `/home/user/.android/avd`, and the trailing space on
the empty `Target:` line dropped.

### "pid alive" without a new port

A `ProcessRunner` can't test an arbitrary pid for liveness (no `kill -0`, no `/proc` access in the
port surface). `reconcile` uses **"the AVD is not in `adb devices`"** as the liveness signal
instead — which is also exactly the ground truth the dashboard cares about. So a `Booting`/`Running`
row whose emulator has vanished from adb (app SIGKILLed mid-boot, emulator later died or was killed)
is reset to `Stopped` with `pid` cleared. If the emulator is genuinely still orphaned and running,
adb still sees it and `reconcile` keeps it `Running`/`Booting` — also correct.

### Property test

`reconcile_converges_to_ground_truth_over_random_scenarios`: xorshift64 (no `proptest` dep — the
project has none and "keep it simple" stands), 48 iterations, fresh provider + tempdir registry per
iteration so the `FakeProcessRunner`'s consume-on-match rule queue only has to answer one
`reconcile()`. `emu avd name` rules are registered in running-serial order (reconcile calls them in
`adb devices` order); the `getprop` rules are all identical (`"1\n"`) so their call order doesn't
matter. Nothing surfaced — it passed first run and every run since.

### No testing-fake changes

`FakeProcessRunner` / `InMemoryFs` were enough as-is. `adopt_row` reads `config.ini` through the
`Fs` port, so `InMemoryFs::write_atomic` seeds it in the enrichment test.
</content>
