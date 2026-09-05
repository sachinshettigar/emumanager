---
id: "0017"
title: "IPC + Create wizard screen + Dashboard wired to real Provider"
milestone: "M2"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Wire tasks `0014`-`0016` through `tauri-specta` commands + one job event into the Create wizard
and Dashboard, so an emulator can be created, booted, and stopped entirely from the UI.

## Context / links

- `src-tauri/src/commands/toolchain.rs` — the pattern followed (thin command, typed event,
  `pub(crate)` module, catalog fetched in the shell via `reqwest`)
- `docs/design/wireframes/` screens 1 and 2

## Scope — files touched

- `src-tauri/src/commands/emulator.rs` (new) — 6 commands + `EmulatorJob` event
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (registration)
- `src/lib/ipc.ts`, `src/lib/bindings.ts` (regenerated)
- `src/routes/Create.tsx` (rewritten), `src/routes/Create.test.tsx` (new)
- `src/routes/Dashboard.tsx` (rewritten), `src/routes/Dashboard.test.tsx` (rewritten)
- `crates/emu-android/src/devices.rs` (`parse_from_jar`, `zip` dep),
  `crates/emu-android/src/provider.rs` (`sdk_root` made `pub`, `is_image_installed`,
  `tracked_states` / `TrackedEmulator`), `crates/emu-android/Cargo.toml`
- `crates/emu-core/src/registry/open.rs` (`list_emulators`)

## Acceptance criteria

- [x] Create wizard: device picker (searchable), image picker (installed / download size),
      hardware form (name/RAM/storage), review → "Create" / "Create & launch", live job log
- [x] Dashboard lists tracked emulators with live Stopped/Booting/Running (polled every 4 s) and
      a per-row Launch/Stop action
- [x] `just bindings` clean once committed; Vitest covers loading/error/success for both screens
      (22 web tests, +9); 3 new Rust unit tests in `commands::emulator`
- [x] `just validate` green (the pre-commit hook stages the regenerated `bindings.ts`)

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### Commands (`src-tauri/src/commands/emulator.rs`)

`list_devices` / `list_images` / `list_emulators` / `create_emulator` / `launch_emulator` /
`stop_emulator`, plus one `EmulatorJob` event (`job://emulator`, tagged `Progress`/`Log`/`Done`
payload with a `jobId` — mirrors `toolchain::BootstrapProgress` exactly). Every command builds an
`AndroidProvider` **per call** (same as `toolchain.rs` builds its ports per call), documented at
the top of the file: the consequence is the provider's spawned-child map (task `0016`) doesn't
persist, so `stop` runs the graceful `adb emu kill` path without a held handle. Fine for M2's
"create → boot → stop"; a shared/managed provider is M3.

### `list_devices` / `list_images` are done in the shell, not on the `Provider` trait

`Provider::list_devices`/`list_images` stay `NotImplemented`. The trait signatures can't take
manifest bytes or a `Downloader`, and adding `reqwest` to `emu-android` would break "network
behind a port". So the commands fetch + parse directly — `devices::parse_from_jar` on the bytes of
the installed `sdklib` jar (read from disk), `sysimg::parse` on the six `sys-img2-3.xml` manifests
fetched concurrently with `futures_util::try_join_all` — exactly as `toolchain::resolve_catalog`
already does for the component catalog. Whether those methods belong on the trait at all is an
M3 question.

### `devices::parse_from_jar` + `zip` on `emu-android`

New pure function: opens the `sdklib` jar (a plain zip), reads each `DEVICE_XML_RESOURCES` entry,
`parse`s it, concatenates. A missing entry is skipped (Google moves these across revisions), a
corrupt jar / malformed XML is an error. Synchronous — no `zip` type is held across an `.await`
(the task-0012 `Send` gotcha). `zip` added to `emu-android` (`default-features=false`,
`deflate` — same knobs `emu-core` uses). The command tries three known jar-relative paths
(`lib/sdklib/sdklib.core.jar` first) since the exact one has moved between `cmdline-tools`
revisions.

### `AndroidProvider` additions

- `sdk_root` made `pub` (the command needs it to find the jar).
- `is_image_installed(coord)` — the `source.properties` marker check, reused for the image
  picker's "installed" badge.
- `tracked_states() -> Vec<TrackedEmulator>` — registry rows + a live `RunState` probed from
  `adb` (`emulator_serials` → `emu avd name` match → `getprop sys.boot_completed`). A lightweight
  read-only view for the Dashboard; **not** `reconcile()` (no adoption of out-of-band AVDs, no
  dropping of vanished rows — M3).
- `Registry::list_emulators()` on the emu-core side.

### Wizard scope decisions

- Hardware step is name + RAM + storage only. Graphics / snapshots / cold-boot / device-frame /
  keyboard / network (spec §5.2) all have sane `Hardware::default()` values and aren't needed for
  M2's DoD — a fuller hardware form is a later pass.
- "Inline download" from the image picker is folded into "Create": the button runs `ensure_image`
  first (streaming a "this can take minutes" log line), rather than a separate download button.
- "Save as profile" (spec §5.2) is M4's `EmuProfile` work — not here.

### specta quirk

`f32` (`DeviceInfo.diagonalIn`) exports as `number | null` from `specta-typescript` 0.0.12 — the
frontend just tolerates the `null`. Not worth a workaround for one cosmetic field.

### Honest gaps (carried from 0015/0016)

The whole flow is fake-driven-tested (Rust) + Vitest (UI); no test boots a real emulator. The
`ensure_image` progress is a coarse log line, not `sdkmanager`'s own byte progress (streaming that
is a follow-up). "App exit doesn't orphan" is partial — see the per-command-provider note above.
All three are M3 / a later polish pass, documented in `MILESTONES.md`'s M2 section.
