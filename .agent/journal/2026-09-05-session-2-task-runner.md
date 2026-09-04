# 2026-09-05 — session 2 (Claude Code)

## Worked on

- Task(s): `0006` (justfile + package.json script mirror + scripts/)
- Milestone: `M0`

## Changed

- `justfile` — header now documents the Windows/WSL path; every recipe has a `just --list`
  description; `bindings` recipe rewritten (was `cargo run -p app` — no such crate) as a bash
  block that runs `cargo test -p emumanager_lib export_bindings` when it exists and otherwise
  prints a skip and exits 0 (real export = task 0004); `test-rust` picks nextest-or-cargo;
  `test-web` no-ops until `node_modules` exists.
- `package.json` — new. Scripts-only mirror: `dev`, `build`, `typecheck`, `lint`, `format:check`,
  `format`, `test`, `e2e`, `tauri`, `validate` (→ `just validate`). No deps yet — task 0003 adds
  Vite/React/TS + eslint/prettier/vitest and runs `pnpm install`.
- `scripts/validate.sh`, `scripts/check-fast.sh` — web steps now guarded on
  `[[ -f package.json && -d node_modules ]]`; a bare `package.json` prints a clear skip instead
  of failing `pnpm typecheck`.
- `Makefile` — `TARGETS` widened to the full recipe set (was a subset).
- `.agent/tasks/0006-*.md` — acceptance criteria ticked, notes added, `status: done`.
- `.agent/state.json`, `MILESTONES.md`, `PROGRESS.md` — 0006 marked done; M0 justfile box ticked.

## State now

- `just --summary`: pass (exit 0) · `just progress`: pass · `just check-fast`: pass
  (`emu-core` test green, web skipped) · `just bindings` / `just test-web`: pass (skip messages)
- `just validate`: still not fully green — pre-0007. Web steps skip (no `node_modules`); the
  rust `just bindings && git diff` step and several lint CLIs (cargo-deny, actionlint, gitleaks,
  markdownlint) are not installed locally. Task 0007 wires + installs all of it.
- Tasks moved: `0006` todo → doing → done.
- `lastValidatedCommit` in state.json: null (still waiting on 0007's real gate).

## Next action

Task `0003` — Vite + React 18 + TypeScript (strict) app shell with the 4 nav routes
(Dashboard / Create / Profiles / Dependencies), `src/styles/tokens.css` mirroring the wireframe
token system, Vitest + Testing Library set up, Vite `build.outDir` → `../dist`. Then `pnpm install`
so `just validate`'s web half goes live. After that: `0002` (emu-core ports/models).

## Gotchas / notes for the next agent

- `package.json` `packageManager` is pinned to `pnpm@10.0.0` — bump to match the installed pnpm
  (11.25) in task 0003 if `corepack` complains, or leave it (pnpm honors it loosely).
- Task 0003 will overwrite `dist/index.html` (the committed placeholder) with the Vite build.
  `.gitignore` already keeps `dist/*` except `!dist/index.html`; either commit the built
  `index.html` or relax the ignore to keep a real build artifact out of git — decide in 0003.
- The `bindings` recipe currently keys on a test named `export_bindings` in crate
  `emumanager_lib`. Task 0004 must create exactly that (or update the recipe) so
  `just bindings` + the CI drift check line up.
- `just validate` runs `cargo sqlx prepare --check --workspace` — will fail until task 0005 adds
  sqlx + `.sqlx/`. Expected.
