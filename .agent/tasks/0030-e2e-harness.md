---
id: "0030"
title: "E2E harness — tauri-driver + WebdriverIO skeleton"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-07"
updated: "2026-09-08"
---

## Goal

Stand up the E2E harness that M2/M3/M4/M5's deferred "real boot" DoD lines all point at
(`tauri-driver` with WebdriverIO), one smoke spec, wired into `just e2e` and a CI job — so the
moment GitHub Actions is unblocked, the deferred DoDs are one flip away, not a from-scratch build.

## Context / links

- `MILESTONES.md` M6 bullet 4; `docs/architecture.md` §6 — the E2E row names Playwright for the UI
  and `tauri-driver` + WebdriverIO for the full Linux/Windows path
- `justfile` already has a `just e2e` that "skips cleanly when no `playwright.config.*` exists"
  (session-3 journal) — replace the skip with a real runner
- `.github/workflows/nightly-integration.yml` runs `just e2e` and currently no-ops
- `tauri-driver`: <https://v2.tauri.app/develop/tests/webdriver/>

## Scope — files this task may touch

- `e2e/` (new) — `wdio.conf.ts`, `tsconfig.json`, `specs/smoke.e2e.ts`
- `package.json` (a `test:e2e` script; `@wdio/*` + `tauri-driver` dev-deps), `justfile` /
  `scripts/` (`just e2e` runs the real thing on Linux/Windows, prints a "macOS: manual checklist"
  line on macOS)
- `.github/workflows/ci.yml` or `nightly-integration.yml` (an `e2e` job — Linux + Windows, builds
  a debug binary, runs `tauri-driver`)
- `docs/testing-and-validation.md` (the E2E row), `docs/playbooks/` (a manual macOS checklist)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `e2e/specs/smoke.e2e.ts` — launches the built app via `tauri-driver`, waits for the
      Dashboard, asserts the "My emulators" heading + the sidebar render, and that
      `data-testid="backend-status"` eventually shows a `pong` (the typed-IPC seam is live). No
      emulator creation yet — that spec is a follow-up once the harness runs green in CI.
- [x] `wdio.conf.ts` starts/stops `tauri-driver`, points `capabilities` at the debug binary path
      per OS, sane timeouts.
- [x] `just e2e`: on Linux/Windows runs `wdio`; on macOS prints "E2E via tauri-driver is
      Linux/Windows only — see docs/playbooks/macos-e2e-checklist.md" and exits 0 (so it never
      fails the nightly on a Mac). **Not** part of `just validate`.
- [x] A CI job (`e2e`, `ubuntu-latest` + `windows-latest`) that builds the app and runs `just e2e`.
      `actionlint` clean. It will not *run* until billing is fixed — that's expected and noted.
- [x] `docs/playbooks/macos-e2e-checklist.md` — the manual steps for a Mac (launch, create + boot a
      Play Store x86_64 emulator, stop it) that M2's DoD describes.
- [x] `just check-fast` then `just validate` green (the e2e deps/config don't touch the Rust or the
      unit-test web build; `knip` must not flag the `e2e/` tree — add it to `knip.json` ignores).

## Validate

```
just validate
actionlint .github/workflows/*.yml
just e2e   # on this Mac: prints the checklist pointer and exits 0
```

## Notes / findings

- **`e2e/` is a fully separate npm project** — its own `package.json` + `package-lock.json` +
  `node_modules` (gitignored), NOT part of the root pnpm workspace. That's the cleanest way to
  keep the wdio dep tree from touching `just validate`: root `tsc -b` only sees `src`, and
  `eslint`/`prettier`/`knip` are told to skip `e2e/`.
- **Excludes added**: `eslint.config.js` `ignores` += `e2e/**`; `.prettierignore` += `e2e/`;
  `.markdownlint-cli2.yaml` `ignores` += `**/node_modules/**` (nested `node_modules` weren't
  excluded before — `e2e/node_modules` is the first nested one in the repo). `knip.json` needed
  no change (its `project` is `src/**` + `scripts/*.mjs`).
- **`tauri-driver` is a cargo binary** (`cargo install tauri-driver --locked`), not npm — so it's
  installed in the CI job / `scripts/e2e.sh` checks for it, not a `package.json` dep.
- **macOS**: `scripts/e2e.sh` detects Darwin, prints the `docs/playbooks/macos-e2e-checklist.md`
  pointer, exits 0. Verified locally (`just e2e` → pointer, exit 0).
- **The wdio deps DO resolve and typecheck** — ran `npm install` in `e2e/` (493 pkgs) +
  `tsc --noEmit -p e2e/tsconfig.json` clean. Not run against a real `tauri-driver` (macOS box,
  and no billing to run CI) — the spec assertions are written against the real DOM
  (`<h1>My emulators`, `<aside>` with "Dependencies", `[data-testid="backend-status"]` → `pong`).
- **CI**: `e2e` job added to `ci.yml` (ubuntu + windows, `cargo install tauri-driver`, Linux also
  `apt-get install webkit2gtk-driver`, then `just e2e`). `nightly-integration.yml`'s old
  "skip if no playwright.config" step replaced with the real `just e2e`. Both `actionlint`-clean.
  Neither runs until the GitHub Actions billing block is lifted — expected, in the criteria.
- `pnpm tauri build --debug --no-bundle` builds `src-tauri/target/debug/emumanager` (bin name =
  Cargo package name, since `tauri.conf.json` sets no `mainBinaryName`). `wdio.conf.ts` tries a
  small candidate list so a future rename doesn't silently break it.
