# 2026-09-08 — session 10 (Claude Code)

## Worked on

- Task `0034` — live download / run progress (8-item batch item #3). After `0032` (rename) and
  `0033` (uninstall).
- Milestone: `M6`.

## Changed

- `crates/emu-core/src/toolchain/bootstrap.rs` — new `run_streamed` helper (spawn + drain
  `next_line`, forward each line to `job`); `accept_licenses` / `install_packages` use it and
  each report a `phase` first. `process_err` helper (dedupe). Tests: `.on_spawn` for the
  `sdkmanager` calls; assert a forwarded line + the license phase reach the collector.
- `src/lib/ipc.ts` — `applyBootstrapEvent` + `applyEmulatorJobEvent`: a changed `phase` with
  `pct == null` now clears the stale `pct` (indeterminate bar, not a frozen number).
- `src/routes/Dependencies.tsx` — `RunProgress` `bootstrap-bar`: determinate fill at `pct`% or a
  full-width `animate-pulse` bar; `data-indeterminate` reflects which.
- `src/routes/Dashboard.tsx`, `src/routes/EmulatorDetail.tsx` — pulsing amber dot + "booting…"
  label while `state === "booting"`.
- `src/routes/Dependencies.test.tsx` — the streaming test also asserts the bar (42% →
  indeterminate on a pct-less phase).
- `.agent/state.json` (task 0034 + note), `PROGRESS.md`, `.agent/tasks/0034-*.md`.

## State now

- `just validate`: **pass** (160 rust tests, 40 web tests, all lints). No `bindings.ts` change —
  0034 adds no `#[tauri::command]`.
- `just progress`: pass (M6, 34 tasks / 34 files).
- Tasks moved: `0034` (new) todo→review.
- `lastValidatedCommit`: set after the commit.

## Next action

Task `0035` — device skins. `LaunchOpts` (`crates/emu-core/src/provider.rs`) already has
`device_frame: bool`; wire it through `AndroidProvider::launch` (`crates/emu-android/src/provider.rs`)
to a real `emulator` arg. Skins ship inside the `emulator` package under
`<sdk>/emulator/resources/<name>.<ext>` and per-device skin dirs under
`<sdk>/skins/`; the `-skin <WxH>` / `-skin <name>` / `-skindir <path>` flags select one. Cite
the real flag set from `emulator -help-skin` / `emulator -help-skindir` (capture into a fixture)
before wiring — don't guess. Then expose a "device frame" toggle in the Create wizard / detail
hardware form.

## Gotchas / notes for the next agent

- **Deliberately no `sdkmanager` %-parsing.** `sdkmanager`'s piped progress format isn't captured
  anywhere in this repo and AGENTS §6.2 forbids guessing SDK output. `run_streamed` forwards raw
  lines only. If someone captures a real `sdkmanager platform-tools` piped transcript into a
  fixture, `run_streamed` is the single place to add a parser that emits `Progress { pct }`.
- **The bar is honestly two-mode:** determinate during the `cmdline-tools` archive download
  (`NativeDownloader` → `pct_progress`, already wired through `run_bootstrap`'s sink), then
  indeterminate during the `sdkmanager` phase. That's the true state of knowledge, not a
  limitation to paper over.
- `FakeProcessRunner::on_spawn(needle, lines, output)` scripts a streamed child; `on_run` and
  `on_spawn` rules are separate queues, both matched by substring on the rendered command line,
  both consumed on match. Bootstrap tests now need `.on_spawn` for `sdkmanager`.
- clippy `await_holding_lock` also fires on a `MutexGuard` kept alive to a later `drop()` even
  with no `.await` between — compute what you need from the guard inside a `{ }` block instead.
