---
id: "0034"
title: "Live download / run progress — stream sdkmanager output + a real progress bar"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

While the app is downloading the SDK or booting an emulator, show what's happening as it happens.
Third of the 8-item feature batch (item #3: "while downloading & running — show status,
progress etc").

## Context / links

- User request (session 10): "Also while downloading & running - show status, progress etc."
- `crates/emu-core/src/toolchain/bootstrap.rs` — `accept_licenses` / `install_packages` used
  `ProcessRunner::run` (blocking), so `sdkmanager`'s output — including the multi-minute
  `emulator` (~900 MB) download — landed in one lump at the end.
- `ProcessRunner::spawn` → `ChildProcess::next_line()` already exists (used by the emulator
  launch path) — streaming is a port that's already there.
- **No `sdkmanager` progress-bar fixture exists** and AGENTS.md §6.2 forbids inventing SDK output
  formats, so this task forwards `sdkmanager`'s own lines verbatim and does **not** parse a
  percentage out of them. The one real percentage we have is the `cmdline-tools` archive
  download, which `NativeDownloader` already reports byte-wise.

## Scope — files this task may touch

- `crates/emu-core/src/toolchain/bootstrap.rs` — `run_streamed` helper; `accept_licenses` /
  `install_packages` stream their output to `job` line by line + report a `phase`
- `src/lib/ipc.ts` — `applyBootstrapEvent` / `applyEmulatorJobEvent`: a new phase with no `pct`
  clears the stale `pct` (so the bar goes indeterminate instead of freezing on an old number)
- `src/routes/Dependencies.tsx` — `RunProgress` gets a real progress bar (determinate when `pct`
  known, indeterminate pulse otherwise)
- `src/routes/Dashboard.tsx`, `src/routes/EmulatorDetail.tsx` — a pulsing dot + "booting…" label
  while an emulator is in the `booting` state
- `crates/emu-core/src/toolchain/bootstrap.rs` tests — `.on_spawn` instead of `.on_run` for the
  `sdkmanager` calls, + assert lines/phase are forwarded
- `src/routes/Dependencies.test.tsx` — assert the bar (determinate 42% → indeterminate on a
  pct-less phase)
- `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `bootstrap`'s `sdkmanager --licenses` and `sdkmanager <pkgs>` calls run via
      `ProcessRunner::spawn`; every non-empty output line is forwarded to `job` as a `Log` the
      instant it arrives. Each step first reports a `phase` ("Accepting SDK licenses",
      "Downloading & installing platform-tools, emulator"). No output-format parsing.
- [x] `applyBootstrapEvent` / `applyEmulatorJobEvent`: `progress` with a **changed** `phase` and
      `pct == null` resets `pct` to `null`; a `pct`-only update within the same phase still
      carries the last phase.
- [x] Dependencies `RunProgress`: while running, a `bootstrap-bar` — a filled bar at `pct`% when
      known (`data-indeterminate="false"`), otherwise a full-width `animate-pulse` bar
      (`data-indeterminate="true"`).
- [x] Dashboard row + EmulatorDetail header: a pulsing amber dot and the label "booting…" while
      `state === "booting"` (other states unchanged).
- [x] Rust: bootstrap tests updated to `.on_spawn`; a new assertion that a forwarded `sdkmanager`
      line and the "Accepting SDK licenses" phase both reach the job collector. 160 rust tests.
- [x] Web: the streaming-progress test also checks the bar is 42% then goes indeterminate on the
      next pct-less phase. 40 web tests.
- [x] `just validate` green.

## Validate

```
just validate
```

## Notes / findings

- **Why no `sdkmanager` % parsing.** `sdkmanager` rewrites a `[===>  ] 42%` bar in place on a TTY
  and prints differently when piped; there's no captured fixture of the piped form in this repo,
  and AGENTS.md §6.2 says don't guess SDK output. Forwarding the raw lines is honest and already
  a big UX win (you see "Downloading emulator-linux_x64.zip", "Unzipping…" live). If a real
  capture is added later, `run_streamed` is the one place to add a parser.
- **The `cmdline-tools` download already has a real %** via `NativeDownloader` → `pct_progress`;
  that flows through `run_bootstrap`'s sink to `job://bootstrap` unchanged. So the bar is
  determinate during that phase and indeterminate during the `sdkmanager` phase — which is
  exactly the honest picture.
- No new `#[tauri::command]` — `bindings.ts` unchanged.
