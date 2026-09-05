---
id: "0020"
title: "Shared managed provider + startup reconcile + rename/edit/delete/wipe commands"
milestone: "M3"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
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

- [ ] `AndroidProvider` (or a thin `SharedProvider` wrapper: `Arc<AndroidProvider>` + the async
      constructor) built once in `setup`, `app.manage()`d. Construction is async → use
      `tauri::async_runtime::block_on` in `setup` or a `OnceCell` initialised on first command; pick
      one, note which and why.
- [ ] Every existing emulator command (`list_devices` … `stop_emulator`) uses the managed provider
      instead of `provider(&app).await?`. `create_emulator`'s child map now persists, so a later
      `stop_emulator` can force-kill.
- [ ] Startup: `reconcile()` runs once after the provider is built (spawned, non-blocking — a slow
      adb mustn't hold up the window). Log the outcome.
- [ ] `RunEvent::ExitRequested` (or window `CloseRequested`): stop / kill every child still in the
      provider's `running` map so quitting the app never orphans an emulator it started. Best-effort,
      time-boxed.
- [ ] New commands, each thin (validate → provider call → DTO / `IpcError`):
  - [ ] `rename_emulator(id, display_name)`
  - [ ] `edit_hardware(id, ram_mb, storage_mb, graphics)` — updates the row; if the AVD must be
        recreated for it to take effect, do that (`delete avd` keeping data + `create avd`) and say
        so in the result / a job log line
  - [ ] `delete_emulator(id, wipe)` → `Provider::delete`
  - [ ] `wipe_emulator_data(id)` — next launch gets `-wipe-data`, or wipe the userdata now if stopped
  - [ ] `reconcile_now()` → `Provider::reconcile`, returns the refreshed `EmulatorInfo` list
  - [ ] `emulator_detail(id) -> EmulatorDetail` (image coord, api, device, ram/storage, source,
        adb serial, grpc port, avd path, created/updated)
- [ ] `just bindings` clean once committed; `ipc.ts` hooks + Vitest for the new hooks
- [ ] `just check-fast` then `just validate` green

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: block_on-in-setup vs OnceCell decision; how ExitRequested reap is time-boxed; whether
edit_hardware actually needs an AVD recreate or `config.ini` rewrite is enough.)
</content>
