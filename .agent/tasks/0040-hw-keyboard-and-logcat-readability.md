---
id: "0040"
title: "Desktop keyboard in launched emulators + logcat readability"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-10"
updated: "2026-09-10"
---

## Goal

Three fixes from live testing of the 0039 build:

1. **The Mac keyboard didn't type into the emulator.** `avdmanager create avd` writes a
   `config.ini` with no `hw.keyboard` key, and the emulator then defaults a phone AVD to
   `hw.keyboard=no` — only the on-screen keyboard works, host keystrokes are dropped. Android
   Studio's AVD Manager writes `hw.keyboard=yes`; match it.
2. **Logcat level filter was terse** — `I+`, `V+`. Spell it out: `Info+`, `Verbose+`.
3. **Logcat rows weren't visually distinct** — Info/Debug/Verbose were all shades of grey.
   Give each priority its own hue + a small colour legend.

(A 4th ask from the same message — a full HTTP request/response inspector with URL, query
string and body — is *not* in this task. That needs a MITM proxy + a user-installed CA cert
or the `androidx.inspection` agent; it's tracked as an M7 line, see `MILESTONES.md`.)

## Context / links

- `hw.keyboard` is a documented AVD hardware property carried in the AVD's `config.ini`
  (emulator command-line reference, <https://developer.android.com/studio/run/emulator-commandline>).
  There is no `emulator`/`avdmanager` CLI flag to force it — it is purely a `config.ini` value.
- `crates/emu-android/src/provider.rs` — `launch()` already resolves the AVD name and runs
  prerequisite checks before spawning; `avdmanager list avd` output (parsed by
  `parse_avdmanager_list_avd` → `AvdEntry`) carries the on-disk `Path:` of each AVD.
- `src/routes/EmulatorDetail.tsx` — `LogcatTab` renders the level `<select>` and the console;
  `LEVEL_TONE` already maps a priority letter → a Tailwind text colour.

## Scope — files this task touched

- `crates/emu-android/src/provider.rs`
  - `list_avd_entries()` — new; `list_avd_names()` now delegates to it.
  - `ensure_hw_keyboard(avd_dir) -> Option<String>` — rewrites `config.ini` to
    `hw.keyboard=yes` (best-effort; returns a launch-log line when it changed something).
  - `hw_keyboard_fix(config_ini) -> Option<String>` — free fn, the pure rewrite (replace an
    existing `hw.keyboard` line or append; `None` if already `yes`). 3 unit tests.
  - `launch()` — after the prereq checks, look up the AVD's dir and call `ensure_hw_keyboard`;
    emit its note on the job log next to `skin_note`. 1 integration test.
- `src/routes/EmulatorDetail.tsx` — `LEVEL_LABEL` map (`Verbose`/`Debug`/`Info`/`Warn`/`Error`);
  `<select>` options now `{LEVEL_LABEL[lv]}+`; `LEVEL_TONE` reworked to 5 distinct hues
  (E red+bold, W amber, I green, D blue, V faint); a `data-level` attr on each row; a colour
  legend under the console controls.
- `src/routes/EmulatorDetail.test.tsx` — assert the spelled-out labels + per-level colour class.
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md` (M7 inspector line), journal.

## Acceptance criteria

- [x] Launching any emulator ensures `hw.keyboard=yes` in its `config.ini` (added when absent,
      `no` → `yes`, untouched when already `yes`); all other lines preserved. Best-effort — a
      missing/unwritable `config.ini` logs a note and never blocks the launch.
- [x] `hw_keyboard_fix` has unit tests for the append / replace / no-op cases; `launch` has an
      integration test that scripts `avdmanager list avd` + a seeded `config.ini` and asserts the
      rewrite.
- [x] Logcat level dropdown reads `Verbose+ / Debug+ / Info+ / Warn+ / Error+`.
- [x] Each logcat row is coloured by priority (5 distinct hues) and a legend names them.
- [x] Vitest covers the new labels + colours; `just validate` green (bar the expected
      `bindings.ts` diff step until commit — no command signatures changed here, so in fact clean).

## Validate

```
just validate
```

## Notes / findings

- Applied in `launch` (not just `create`) so pre-existing / adopted AVDs get fixed too. Costs
  one extra `avdmanager list avd` per launch — cheap next to booting an emulator, and it fails
  soft (unmatched → empty list → skip).
- `hw.keyboard=yes` makes Android treat a hardware keyboard as connected, which also suppresses
  the on-screen IME in text fields. For a desktop emulator manager that's the desired behaviour
  and is exactly what Android Studio does.
- If an emulator was already booted from a snapshot before the fix, one cold boot
  (`-no-snapshot-load`, the "Cold boot" toggle) may be needed for the new `config.ini` to take.
- No `tracing` in `emu-android` (near-zero deps) — the keyboard note rides the existing job log
  instead of a logger.
