---
id: "0038"
title: "Live device inspector — logcat viewer + device facts"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

Track and view a running emulator's live state — logs (Android-Studio-Logcat-style, filterable),
plus storage / battery / model. Last of the 8-item feature batch (items #1 "track storage /
networks / logs" + #2 "view them live & filter exactly like Android Studio").

## Context / links

- User request (session 10): "track things about the device like storage, networks, logs…";
  "view them live & filter or query them exactly like how it's shown in android studio."
- `adb -s <serial> logcat -v threadtime` — documented parseable format
  (<https://developer.android.com/tools/logcat#outputFormat>):
  `MM-DD HH:MM:SS.mmm  PID  TID L TAG: message`.
- `adb shell getprop <key>`, `adb shell dumpsys battery` (`level:` line), `adb shell df /data`
  (POSIX `Filesystem 1K-blocks Used Available Use% Mounted-on`) — all standard, stable shapes;
  parsed defensively (first plausible token, `None` on any miss).
- `AndroidProvider` already has `serial_for_avd`, a `running` `ChildHandle` map, and a
  `shutdown()` reaper — this task adds a parallel `logcats` map and extends the reaper.

## Scope — files this task may touch

- `crates/emu-android/src/provider.rs` — `logcats` map; `logcat_start` / `logcat_stop` /
  `device_facts` / `running_serial`; `DeviceFacts` struct; `parse_battery_level` / `parse_df_data`
  free fns; `shutdown` also reaps logcats; `ChildHandle` made `pub`
- `src-tauri/src/commands/device.rs` (new) — `DeviceLogLine` event (`device://log`),
  `start_logcat` (spawns a drain task) / `stop_logcat` / `device_facts` commands, `DeviceFactsDto`
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — register the module, 3 commands, 1 event
- `src/lib/bindings.ts` (regen), `src/lib/ipc.ts` — `parseLogcatLine`, `filterLogcat`,
  `LOGCAT_LEVELS`, `useStartLogcat` / `useStopLogcat` / `useLogcatStream` / `useDeviceFacts`
- `src/routes/EmulatorDetail.tsx` — a `DeviceInspector` section: facts strip + logcat viewer
  (start/stop, level / tag / text filters, pause, clear, shown/total count)
- `src/routes/EmulatorDetail.test.tsx` — mocks + a streaming/filter test
- `MILESTONES.md`, `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `AndroidProvider::logcat_start(id) -> ChildHandle` — resolves the running serial, kills any
      existing stream for that id, spawns `adb -s <serial> logcat -v threadtime`, registers the
      handle. `logcat_stop(id)` kills + forgets it (idempotent). `shutdown()` reaps both maps.
- [x] `AndroidProvider::device_facts(id) -> DeviceFacts` — `ro.product.model`,
      `ro.build.version.release`, `ro.build.version.sdk`, battery `level:`, `df /data`
      free/total (MB). Every field `Option`; a parse miss is `None`, never a wrong value.
- [x] `start_logcat` command spawns a background task draining the handle and emitting each line
      as `DeviceLogLine { id, line }` on `device://log`; a `— logcat stream ended —` marker on
      EOF. `stop_logcat`, `device_facts` commands + `DeviceFactsDto`.
- [x] Frontend: `parseLogcatLine` (threadtime regex → level/tag/message; unmatched → level `?`),
      `filterLogcat` (min-level rank V<D<I<W<E, tag substring, free-text substring; a `?` line
      always passes the level gate). `useLogcatStream(id)` accumulates (capped 5000) + `clear`.
      `useDeviceFacts(id, enabled)` polls every 5 s.
- [x] `DeviceInspector` on the detail panel: facts strip (model · Android X · API n · battery % ·
      /data free/total), Start/Stop logcat, level `<select>`, tag + text inputs, Pause (freezes
      the view), Clear, and a "shown / total lines" counter. Level-coloured lines.
- [x] Vitest: "streams logcat and filters it by level and text" — feeds `device://log` events
      (incl. one for another emulator id that's ignored), checks text shows, then raising the
      min level to E drops the Info line. 44 web tests.
- [x] `just validate` green apart from the expected `bindings.ts` diff step. ~170 rust tests.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- **Client-side filtering, like Android Studio.** The stream is unfiltered `logcat`; level / tag /
  text filtering happens in `filterLogcat` on the accumulated lines. Pause freezes a snapshot so
  you can read without the tail jumping.
- **Defensive parsing, no invented formats.** `-v threadtime` has a documented shape; `getprop` /
  `dumpsys battery` / `df` are standard. Each parser takes only the first plausible token and
  yields `None` otherwise — a weird ROM never produces a wrong number, just a blank.
- **"Networks" is only partly covered.** The facts strip does model / battery / storage. A full
  connectivity read (`dumpsys connectivity` / `ip addr`) has a large, version-variable output
  with no fixture here, so it's left for a follow-up (noted on `MILESTONES.md` M7).
- `logcat_start` returns the `ChildHandle`; the **drain loop lives in the command** (`src-tauri`)
  where `tokio::spawn` + `AppHandle` are available. `AndroidProvider` stays runtime-light.
