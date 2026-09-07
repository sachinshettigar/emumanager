---
id: "0033"
title: "Uninstall SDK components (mirror of the per-component install)"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

The Dependencies screen can install each SDK component on its own (task 0031). Add the inverse:
remove one app-managed component with `sdkmanager --uninstall`, from a per-row button with a
confirm step. Second of the 8-item feature batch (item #7).

## Context / links

- User request (session 10): "Similar to ability to install add ability to uninstall as well."
- `crates/emu-core/src/toolchain/bootstrap.rs` — the install path this mirrors; `sdkmanager` is
  invoked the same way (`Command` via `ProcessRunner`, non-zero → `CoreError::Process`).
- `sdkmanager --uninstall <package> --sdk_root=<path>` — documented at
  <https://developer.android.com/tools/sdkmanager> ("Uninstall packages").
- `InstalledState` is scanned fresh from marker files on every `list_components` call, so removing
  the files is all the bookkeeping there is — no separate marker to clear.

## Scope — files this task may touch

- `crates/emu-core/src/model/component.rs` — `ComponentId::from_repo_path` (inverse of `repo_path`)
- `crates/emu-core/src/toolchain/uninstall.rs` (new) — `uninstall()` + `UninstallPorts`
- `crates/emu-core/src/toolchain/mod.rs` — export them
- `src-tauri/src/commands/toolchain.rs` — `uninstall_component(componentId)` command
- `src-tauri/src/lib.rs` — register it
- `src/lib/bindings.ts` (regenerated), `src/lib/ipc.ts` — `useUninstallComponent`
- `src/routes/Dependencies.tsx` — per-row "Uninstall" → "Confirm remove" / "Cancel"
- `src/routes/Dependencies.test.tsx` — mock + two tests
- `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `emu_core::toolchain::uninstall(data_dir, target, state, os, ports, job)` — runs
      `sdkmanager --uninstall <repo_path> --sdk_root=<data_dir>/sdk` for an **app-managed** target.
      Refuses `CmdlineTools` (`Unsupported` — it's the uninstaller itself). Refuses a target found
      only in a **system** SDK (`Unsupported`, names the path). `NotFound` when cmdline-tools
      aren't installed at all. Not-installed target → clean no-op. Non-zero exit → `Process`.
      6 unit tests with `FakeProcessRunner`.
- [x] `ComponentId::from_repo_path("platform-tools") == Some(PlatformTools)`, `None` for unknown.
- [x] `uninstall_component(componentId)` command — resolves the id, re-scans `InstalledState`,
      calls `toolchain::uninstall` with a no-op `JobHandle` (removal is sub-second, no stream).
      `not_found` IpcError for an unknown id. Registered in `collect_commands!`.
- [x] Dependencies: an installed, app-managed, non-cmdline-tools row shows an "Uninstall" button;
      clicking it swaps in "Confirm remove" + "Cancel"; confirm calls `uninstall_component` and
      invalidates the components query so the row flips back to "Install". A system-sourced row
      shows no Uninstall button. `useUninstallComponent()` hook.
- [x] Vitest: "uninstalls an app-managed component after a confirm step" (asserts no call before
      confirm, `uninstall_component("emulator")` after) + "offers no Uninstall for a component
      found in a system SDK". 40 web tests.
- [x] `just validate` green apart from the `git diff src/lib/bindings.ts` step (lefthook stages
      the regen on commit — documented behaviour). 160 rust tests.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- **Never touches a system SDK.** `uninstall` only acts when `ComponentLocation.source` is
  `AppManaged`. A component from `ANDROID_HOME` / Android Studio is reported but the button is
  hidden client-side *and* the core fn refuses server-side — defence in depth.
- **`--sdk_root=` is passed explicitly** so the removal targets the app-managed SDK even when the
  `sdkmanager` binary being run belongs to a system cmdline-tools install.
- No progress stream: `sdkmanager --uninstall` just deletes a directory, sub-second. The command
  returns `Result<(), IpcError>` and the row shows a "Removing…" label while the mutation is in
  flight. Real download/run progress is task 0034.
