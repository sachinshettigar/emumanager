---
id: "0035"
title: "Device skins / frame — parse <d:skin>, pass -skin at launch, expose a toggle"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

Emulators launch with just a screen — no bezel. Make the device frame real: read each device
profile's skin, pass `-skin` to the `emulator` binary when the frame is wanted, and give the user
a checkbox. Fourth of the 8-item feature batch (item #4: "I'm not seeing proper skins of devices,
add them").

## Context / links

- User request (session 10): "I'm not seeing proper skins of devices can you add them AS WELL."
- `crates/emu-core/src/model/emulator.rs` — `Hardware.device_frame: bool` already existed
  (defaults `true`) but was **never used** anywhere. This task makes it load-bearing.
- `<d:skin>` is a real element in `devices.xml` / `nexus.xml` / `wear.xml` / `tv.xml` /
  `automotive.xml` (fixtures under `crates/emu-android/tests/fixtures/`).
- `-skin <name>` + `-skindir <dir>` are documented at
  <https://developer.android.com/studio/run/emulator-commandline> (already the citation in
  `AndroidProvider::launch`'s doc comment).

## Scope — files this task may touch

- `crates/emu-core/src/model/device.rs` — `DeviceProfile.skin: Option<String>`
- `crates/emu-core/src/provider.rs` — `LaunchOpts.skin: Option<String>`
- `crates/emu-android/src/devices.rs` — parse `<d:skin>` (+ tests)
- `crates/emu-android/src/provider.rs` — `launch` passes `-skin`/`-skindir` when `opts.skin` is
  set **and** `<sdk>/skins/<name>` exists; a missing skin is noted on the job log, not fatal
  (+ 2 tests)
- `src-tauri/src/commands/emulator.rs` — `DeviceInfo.skin`; `CreateEmulatorRequest.device_frame`;
  `EmulatorDetail.device_frame`; `edit_hardware(device_frame)`; `device_skin` + `launch_opts_for`
  helpers wire it into `launch_emulator` and the create-and-launch path
- `src/lib/bindings.ts` (regen), `src/lib/ipc.ts` — `EditHardwareVars.deviceFrame`
- `src/routes/Create.tsx` — "Show device frame (bezel)" checkbox + review row
- `src/routes/EmulatorDetail.tsx` — "Device frame" checkbox in the hardware form
- `src/routes/Create.test.tsx`, `src/routes/EmulatorDetail.test.tsx` — fixture fields + assertion
- `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `DeviceProfile.skin` parsed from `<d:skin>` (child of `<d:hardware>`); `pixel_6` →
      `Some("pixel_6")`, a device with no `<d:skin>` → `None`. 2 device tests.
- [x] `LaunchOpts.skin`; `AndroidProvider::launch` appends `-skin <name> -skindir <sdk>/skins`
      only when `<sdk>/skins/<name>` exists on disk, else logs
      "device-frame skin '<name>' isn't installed … launching without a frame". 2 launch tests.
- [x] `launch_emulator` + create-and-launch build `LaunchOpts` via `launch_opts_for`: `skin` is
      set from the device profile's `<d:skin>` when the emulator's stored `device_frame` is on.
      Any lookup failure → no frame, launch still proceeds.
- [x] `CreateEmulatorRequest.device_frame` (Create wizard checkbox, default on) flows into
      `Hardware.device_frame`. `EmulatorDetail.device_frame` shown; `edit_hardware` takes and
      persists it (detail-panel checkbox).
- [x] `just validate` green apart from the expected `bindings.ts` diff step. 166 rust, 40 web.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- **Skins aren't bundled with cmdline-tools.** `<sdk>/skins/` only exists once a skin-carrying
  package is installed (or Android Studio put it there). So the checkbox can be on and the frame
  still not appear — the launch log says exactly why ("skin 'pixel_6' isn't installed under
  …/skins"). This is honest and matches the pre-existing behaviour (no frame) rather than
  regressing it. A future task could fetch skin packs.
- **`device_frame` was dead state until now** — stored in the registry, defaulted `true`, read by
  nothing. `launch_opts_for` in `commands/emulator.rs` is the single place that turns it into a
  real `-skin` argument, by re-reading the `sdklib` jar (same source as `list_devices`) to get
  the profile's skin name. No registry migration — the skin name is looked up at launch, not
  stored.
- No AVD-config (`skin.name` / `showDeviceFrame`) writing: `avdmanager create -d <device>`
  already records the device's skin in `config.ini`; the missing piece was the launch flag.
