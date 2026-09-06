# 2026-09-06 — session 9 (continued) (Claude Code)

## Worked on

- Task `0027` (done, in review): Dependencies host panel + launch-gating. **M5 functionally
  complete** (`0025`–`0027`).
- `currentMilestone` → M6.

## Changed

- `src/lib/ipc.ts` — `useHostReport` / `useRunHelper`.
- `src/routes/Dependencies.tsx` — `HostPanel` (verdict banner, tiles, per-fix "Fix it").
- `src/routes/Dashboard.tsx` / `EmulatorDetail.tsx` — Launch disabled + reason under `cannotRun`.
- `src/routes/{Dependencies,Dashboard,EmulatorDetail}.test.tsx` — `probeHost` / `runHelper` mocks;
  3 new tests.
- `MILESTONES.md` (M5 boxes), `PROGRESS.md`, `.agent/state.json`, `.agent/tasks/0027`.

## State now

- `just validate`: **pass** (154 rust tests, 34 web tests). `bindings.ts` unchanged.
- `just progress`: pass (milestone M6).
- Tasks `0025`–`0027` review. M5 `in_progress` on the live-CI DoD line.
- `lastValidatedCommit`: set after pushing.

## Next action — M6 is largely CI-gated

M6 (Cross-platform hardening & packaging) DoD: CI builds signed + notarized installers for all
three OSes from a tagged pre-release, the macOS build shows no Gatekeeper warning, and the app
auto-updates from the previous pre-release. **None of that is reachable from code alone** while
GitHub Actions is billing-blocked (a human must fix Settings → Billing), and signing needs real
Apple / Windows certificates in CI secrets.

Buildable slices that don't need a green CI run:
- **Least-privilege Tauri v2 capability audit** — `src-tauri/capabilities/default.json` is still
  just `core:default`; scope it to exactly the commands/events the frontend uses, no wildcard
  fs/shell.
- **`release.yml` + updater config** — write the workflow + `tauri.conf.json` updater block so a
  tag *would* produce update artifacts; `actionlint` it; it just can't be *run*.
- **The E2E harness** (`tauri-driver` + a Playwright/WebdriverIO spec) that M2/M3/M4's deferred
  DoDs all point at — write it, mark it `--ignored` / not in `just validate`, so the moment CI is
  unblocked it's one flip.
- **Rotating logs + "export diagnostics" command** (redacted) — pure code.
- **`cargo deny` / arch-coverage notes**.

Recommend scoping M6 as: `0028` capability audit + `release.yml` + updater config (+ `actionlint`);
`0029` rotating logs + `export_diagnostics`; `0030` the E2E harness skeleton. Then M6 stays
`in_progress` on the CI-run DoD exactly as M0 does.

## Gotchas / notes for the next agent

- **`HostReportDto` is flat strings** — match `"cannotRun"` etc., not enums.
- **`degraded` is not gated** — deliberate; only `cannotRun` blocks Launch.
- **`useHostReport` fires on mount everywhere** — any new screen test needs the `probeHost` mock.
- M5's Windows `emu-helper` bodies + `run_helper` elevation are **untested** — that's part of what
  M6's Windows CI runner is for.
- CI still blocked on GitHub billing — the through-line for M0, M2, M3, M4, M5, and M6's DoDs.
