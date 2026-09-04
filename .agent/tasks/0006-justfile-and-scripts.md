---
id: "0006"
title: "justfile + package.json script mirror + scripts/"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-04"
---

## Goal

Every command in `AGENTS.md` §4 exists as a `just` recipe, mirrored by `package.json` scripts,
delegating to small POSIX + node scripts in `scripts/`.

## Scope — files this task may touch

- `justfile`
- `package.json` (`scripts` block)
- `scripts/validate.sh`, `scripts/check-fast.sh`, `scripts/setup.sh`, `scripts/progress-check.mjs`,
  `scripts/emu-core-no-tauri.sh`
- `Makefile` (thin delegator to `just`)

## Acceptance criteria

- [x] Recipes: `setup`, `dev`, `check-fast`, `validate`, `bindings`, `db-migrate`, `db-prepare`,
      `test`, `test-rust`, `test-web`, `test-integration`, `e2e`, `progress` (+ `schema-check`)
- [x] `just` with no args prints the recipe list; `just --summary` exits 0
- [x] `package.json` has `typecheck`, `lint`, `format:check`, `test`, `build`, `validate`
      (the last shelling to `just validate`)
- [x] Scripts work on macOS + Linux; Windows path (`just` on Git Bash / WSL) documented in the
      justfile header. Aggregator scripts use `set -uo pipefail` (not `-e`) deliberately — they
      run every step and collect failures; `setup.sh` / `emu-core-no-tauri.sh` use `-euo pipefail`.
- [x] `scripts/emu-core-no-tauri.sh` greps `crates/emu-core/Cargo.toml` + `cargo tree` and
      exits non-zero if `tauri` appears
- [x] `scripts/progress-check.mjs` (node, zero deps): equivalent structural checks to
      `.agent/state.schema.json` (project/updated/currentMilestone/milestones/tasks shape + enums),
      every `tasks[].id` has a matching `.agent/tasks/<id>-*.md` whose front-matter `status`/`id`
      equal the json, `currentMilestone` exists, one `doing` max, any `done` milestone has all its
      `MILESTONES.md` boxes ticked
- [x] `just progress` runs it and passes against the current repo

## Validate

```
just --summary && just progress
```

## Notes / findings

- Most of this task's plumbing already existed from the session-0 scaffold; this pass finished it:
  added `package.json` (scripts-only mirror; deps land in task 0003), fixed the `bindings` recipe
  (was `-p app`; now a graceful no-op keyed on `emumanager_lib` until task 0004), added Windows
  guidance to the justfile header, gave every recipe a `just --list` description, and guarded the
  web steps in `validate.sh` / `check-fast.sh` on `node_modules` so a bare `package.json` does not
  break the gate before `pnpm install`.
- `Makefile` target list widened to the full recipe set.
- Deviation from the acceptance text: aggregator scripts stay `set -uo pipefail` (no `-e`) on
  purpose — they must run every step and report all failures, not abort on the first.
- `progress-check.mjs` does not literally `ajv`-validate against `state.schema.json` (zero-dep
  rule); its hand-rolled checks cover the same shape and more. Schema file kept as the spec.
