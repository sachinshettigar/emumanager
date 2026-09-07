# 2026-09-08 — session 10 (Claude Code)

## Worked on

- Task `0033` — uninstall SDK components (8-item batch item #7). Follows `0032` (rename).
- Milestone: `M6`.

## Changed

- `crates/emu-core/src/model/component.rs` — `ComponentId::from_repo_path` (inverse of `repo_path`).
- `crates/emu-core/src/toolchain/uninstall.rs` (new) — `uninstall()` + `UninstallPorts`; 6 unit
  tests with `FakeProcessRunner`.
- `crates/emu-core/src/toolchain/mod.rs` — export `uninstall` / `UninstallPorts`.
- `src-tauri/src/commands/toolchain.rs` — `uninstall_component(componentId)` command (no progress
  stream — removal is sub-second; `JobHandle::noop`).
- `src-tauri/src/lib.rs` — register it. `src/lib/bindings.ts` regenerated.
- `src/lib/ipc.ts` — `useUninstallComponent` (+ `uninstallComponent` wrapper).
- `src/routes/Dependencies.tsx` — per-row "Uninstall" → "Confirm remove" / "Cancel"; hidden for a
  system-sourced component and for `cmdline-tools;latest`. Inline error line on the row.
- `src/routes/Dependencies.test.tsx` — `uninstallComponent` mock, `EMULATOR_INSTALLED` /
  `PLATFORM_TOOLS_SYSTEM` fixtures, 2 new tests.
- `.agent/state.json` (task 0033 + note), `PROGRESS.md`, `.agent/tasks/0033-*.md`.

## State now

- `just validate`: **pass** apart from the `git diff --exit-code src/lib/bindings.ts` step, which
  fails locally until commit (lefthook pre-commit regenerates + stages it — documented, expected).
  160 rust tests, 40 web tests, fmt/clippy/typecheck/lint/prettier/markdownlint all clean.
- `just progress`: pass (M6, 33 tasks / 33 files).
- Tasks moved: `0033` (new) todo→review.
- `lastValidatedCommit`: set after the commit.

## Next action

Task `0034` — real download/run progress. `sdkmanager` prints its own progress to stdout
(`[=====>              ]  38% Downloading platform-tools.zip`); parse that in the bootstrap /
`install_component` path and emit real `Progress { pct, phase }` on `job://bootstrap` instead of
the current coarse "installing …" log line. Then surface a progress bar (not just log lines) in
`RunProgress` (Dependencies) and the emulator job panels, and make booting/running state read
more clearly.

## Gotchas / notes for the next agent

- **`uninstall` never touches a system SDK** — it acts only when `ComponentLocation.source ==
  AppManaged`. Defence in depth: the Dependencies row hides the button for a system-sourced
  component, and `emu_core::toolchain::uninstall` also returns `Unsupported` for one. Keep both.
- **`--sdk_root=` is passed explicitly** on the `sdkmanager --uninstall` call so removal targets
  the app-managed SDK even if the `sdkmanager` binary being run is a system cmdline-tools install.
- **`cmdline-tools;latest` is never removable from the UI** — it's the tool doing the removing.
  The id constant `CMDLINE_TOOLS_ID` in `Dependencies.tsx` and the `ComponentId::CmdlineTools`
  guard in `uninstall.rs` both enforce this.
- `useMutation.mutate(id, { onSettled })` — the per-call callback needs a braced body or
  eslint's `no-confusing-void-expression` fails (`setConfirming(false)` returns void).
- The bindings-diff `just validate` "failure" is normal for any task that adds/changes a
  `#[tauri::command]`; it clears on `git commit` (lefthook stages the regen). The pre-push hook
  runs the full `just validate` again and that one passes.
