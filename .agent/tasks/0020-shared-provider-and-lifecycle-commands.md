---
id: "0020"
title: "Shared managed provider + startup reconcile + rename/edit/delete/wipe commands"
milestone: "M3"
status: "review"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-06"
---

## Goal

Stop building an `AndroidProvider` per command. Construct one in `setup`, hold it as
`tauri::State`, so its spawned-child map survives between calls (real force-kill from the UI) and the
app can reap children on exit ("app exit doesn't orphan"). Reconcile once on startup. Add the
lifecycle commands the detail panel (task `0021`) needs: `rename` / `edit_hardware` / `delete` /
`wipe_data` / `reconcile_now` / `emulator_detail`.

## Context / links

- Milestones: `MILESTONES.md` M3 bullets 2 (startup reconcile), 4 (wipe/delete/rename/edit)
- Carried M2 gap: `PROGRESS.md` "app exit doesn't orphan" — per-command provider is why it's partial
- `src-tauri/src/commands/toolchain.rs` builds ports per call — `emulator.rs` copied that; this
  task changes `emulator.rs` only, `toolchain.rs` can stay as-is
- Depends on tasks `0018` (typed rows) + `0019` (`reconcile` / `delete`)

## Scope — files this task may touch

- `src-tauri/src/lib.rs` (construct provider in `setup`, `manage()` it, `RunEvent::ExitRequested` reap)
- `src-tauri/src/commands/emulator.rs` (take `State<'_, SharedProvider>`; new commands)
- `src-tauri/src/commands/mod.rs`
- `src/lib/bindings.ts` (regenerated), `src/lib/ipc.ts` (hooks for the new commands)
- `src/routes/Dashboard.tsx` — only if a hook signature changes
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `ManagedProvider` (`src-tauri/src/provider_state.rs`): `tokio::sync::OnceCell<Arc<AndroidProvider>>`
      + the fixed `(data_dir, os)` inputs, built once in `setup` and `app.manage()`d. **OnceCell on
      first command**, not `block_on` in `setup` — `Registry::open` runs migrations and shouldn't
      block the window; `setup` only records the inputs (`data_dir`/`os` `None` → a clear
      `unsupported` error at first call, app still launches).
- [x] Every emulator command takes `State<'_, ManagedProvider>` and calls `mgr.get().await?`
      instead of the old per-call `provider(&app)`. `create_emulator`'s spawned-child map now
      persists, so a later `stop_emulator` reaches `AndroidProvider::stop`'s force-kill fallback.
- [x] Startup: `reconcile()` runs once, `tauri::async_runtime::spawn`ed from `setup` (non-blocking).
      Best-effort — no logging surface in this crate, so a failure is silently recovered by the 4 s
      Dashboard poll / next action (noted in Notes).
- [x] `RunEvent::ExitRequested` (via `.build(...).run(|handle, event| …)`) → `provider.peek()` and
      `AndroidProvider::shutdown()` (drains the `running` map, kill+wait each child), wrapped in an
      8 s `tokio::time::timeout` + `block_on`.
- [x] New commands (thin: validate → provider call → DTO / `IpcError`):
  - [x] `rename_emulator(id, display_name)` → `AndroidProvider::rename`
  - [x] `edit_hardware(id, ram_mb, storage_mb, graphics)` → `AndroidProvider::set_hardware` —
        **records the row only**; a live `config.ini` rewrite / AVD recreate is deferred (the
        method doc + command doc say "takes effect on next AVD recreate"). Recreate-on-edit is a
        follow-up, out of scope here.
  - [x] `delete_emulator(id, wipe)` → `Provider::delete` (task `0019`)
  - [x] `wipe_emulator_data(id)` → `AndroidProvider::wipe_data`: refuses while running, else deletes
        the AVD's writable images + `snapshots/` (`userdata-qemu.img`, `cache.img`, …) via `Fs` so
        the next launch rebuilds them — the same effect `emulator -wipe-data` has. AVD dir resolved
        from `$ANDROID_AVD_HOME` / `~/.android/avd`.
  - [x] `reconcile_now()` → `Provider::reconcile` then `tracked_states()` → refreshed `EmulatorInfo[]`
  - [x] `emulator_detail(id) -> EmulatorDetail` (avd name, display, device, image coord + api +
        play-store flag, ram/storage, graphics, source label, live state + serial, created/updated
        RFC 3339, `avd_path` for "Open folder")
  - [x] `reveal_path(path)` — `open -R` / `explorer /select,` / `xdg-open`, no new dependency
- [x] `just bindings` clean once committed (16 commands now); `useReconcileNow` hook + a Dashboard
      "Refresh" button + a Vitest for it. **The detail-panel hooks (`emulator_detail` / rename /
      edit / delete / wipe / `revealPath`) land in task `0021`** with the panel that consumes them —
      shipping them here with no consumer would trip `knip`. Their backend commands are done and in
      `bindings.ts`.
- [x] `just check-fast` then `just validate` green (128 rust tests, 23 web)

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### OnceCell, not `block_on` in `setup`

`AndroidProvider::new` is cheap but `Registry::open` runs migrations — not something to block the
window on. `setup` only records `(data_dir, os)` into a `ManagedProvider` and `app.manage()`s it;
the first command that calls `mgr.get().await` builds the provider via `OnceCell::get_or_try_init`.
If the host is unsupported or has no app-data dir, `init` is `None` and `get()` returns a clear
`unsupported` `IpcError` — the app still launches (the Dashboard shows that error instead of a list).

### `edit_hardware` records intent only

It updates the registry row's `hardware_json`. It does **not** rewrite the live AVD `config.ini`
or recreate the AVD, so a RAM/storage change doesn't take effect until the AVD is next recreated.
Doing the recreate (delete-avd-keeping-data + create-avd, or a targeted `config.ini` rewrite) is a
real follow-up — it needs its own captured behaviour for the `config.ini` keys and the
keep-userdata path, and this task was already large. The method + command docs say so plainly;
the detail panel (task `0021`) will surface the same caveat.

### `wipe_emulator_data` = delete the writable images

`avdmanager` has no wipe subcommand and `emulator -wipe-data` needs a boot. The real mechanism
`-wipe-data` uses is: delete `userdata-qemu.img` (+ `.qcow2`), `cache.img`, `snapshots/` from the
AVD dir; the next launch recreates them from the system image's `userdata.img`. So `wipe_data`
does exactly that via the `Fs` port (missing file = fine), refusing while the emulator is running.
AVD dir = `$ANDROID_AVD_HOME` or `~/.android/avd` (+ `%USERPROFILE%`), `<avd_name>.avd`.

### Detail-panel hooks deferred to 0021

`useReconcileNow` ships here (Dashboard "Refresh" button + Vitest). The other five
(`emulator_detail`, rename, edit, delete, wipe) plus `revealPath` would be exported-but-unused →
`knip` fails `just validate`. Their backend commands are complete and in `bindings.ts`; task
`0021`'s panel adds the thin hooks alongside the UI that calls them.

### Exit reap

`.build(context).run(|handle, event| …)` replaced `.run(context)`. On `RunEvent::ExitRequested`,
`handle.try_state::<ManagedProvider>().and_then(|m| m.peek())` (peek — don't build a provider just
to tear it down), then `block_on(timeout(8s, provider.shutdown()))`. `AndroidProvider::shutdown`
drains the `running` map and `kill()` + `wait()`s each child with a per-child 5 s timeout.

### `reveal_path`

Plain `std::process::Command` — `open -R <path>` (macOS), `explorer /select,<path>` (Windows),
`xdg-open <dir>` (Linux — no portable reveal-and-select, so open the containing dir). No
`tauri-plugin-opener` dependency / capability entry needed for one spawn.
</content>
