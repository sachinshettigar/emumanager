# 2026-09-08 — session 10 (Claude Code)

## Worked on

- Task `0035` — device skins / frame (8-item batch item #4). After `0032`–`0034`.
- Milestone: `M6`.

## Changed

- `crates/emu-core/src/model/device.rs` — `DeviceProfile.skin: Option<String>`.
- `crates/emu-core/src/provider.rs` — `LaunchOpts.skin: Option<String>`.
- `crates/emu-android/src/devices.rs` — parse `<d:skin>` (child of `<d:hardware>`); 2 tests.
- `crates/emu-android/src/provider.rs` — `launch` appends `-skin <name> -skindir <sdk>/skins`
  only when `<sdk>/skins/<name>` exists, else emits a job-log note; 2 tests.
- `src-tauri/src/commands/emulator.rs` — `DeviceInfo.skin`, `CreateEmulatorRequest.device_frame`,
  `EmulatorDetail.device_frame`, `edit_hardware(device_frame)`; new `device_skin` +
  `launch_opts_for` helpers used by `launch_emulator` and create-and-launch.
- `src/lib/bindings.ts` regen; `src/lib/ipc.ts` `EditHardwareVars.deviceFrame`.
- `src/routes/Create.tsx` — "Show device frame (bezel)" checkbox + review row.
- `src/routes/EmulatorDetail.tsx` — "Device frame" checkbox in the hardware form.
- `src/routes/Create.test.tsx` / `EmulatorDetail.test.tsx` — fixture fields + assertion.
- `.agent/state.json` (task 0035 + note), `PROGRESS.md`, `.agent/tasks/0035-*.md`.

## State now

- `just validate`: **pass** apart from the `git diff src/lib/bindings.ts` step (lefthook stages
  the regen on commit — expected). 166 rust tests, 40 web tests.
- `just progress`: pass (M6, 35 tasks / 35 files).
- Tasks moved: `0035` (new) todo→review.
- `lastValidatedCommit`: set after the commit.

## Next action

Task `0036` — export-profile parity. Export already exists (`export_profile` command + the
"Export profile" button on `EmulatorDetail.tsx`, clipboard + `<textarea>`; "Save as profile" in
the Create wizard review step). Surface it as a top-level action: a per-row "Export" on the
Dashboard, and make the flow more parallel to import — ideally write the `.emuprofile` to a
user-chosen file rather than only the clipboard (Tauri dialog + fs plugin, or a
`save_profile_to_path` command).

## Gotchas / notes for the next agent

- **`Hardware.device_frame` is now wired but skins may still be absent.** `<sdk>/skins/` only
  exists once a skin-carrying package (or Android Studio) put it there; cmdline-tools alone has
  none. The launch log line "device-frame skin '<name>' isn't installed under …/skins" is the
  honest signal. Not a bug to "fix" by force-passing `-skin` — that just makes the emulator warn.
- **The skin name is looked up at launch, not stored.** `launch_opts_for` re-reads the `sdklib`
  jar (same as `list_devices`) each launch. If that ever gets slow, the fix is to persist the
  skin on the registry row at create time (needs a migration) — deliberately not done now.
- `edit_hardware` gained a positional `device_frame: bool` param — the 5th arg. `bindings.ts`
  and `EditHardwareVars` both updated; `EmulatorDetail.tsx` passes it via `{ id, ...effectiveHw }`
  where `effectiveHw` now includes `deviceFrame`.
