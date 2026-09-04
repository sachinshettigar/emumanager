---
id: "0006"
title: "justfile + package.json script mirror + scripts/"
milestone: "M0"
status: "todo"
owner: ""
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

- [ ] Recipes: `setup`, `dev`, `check-fast`, `validate`, `bindings`, `db-migrate`, `db-prepare`,
      `test`, `test-rust`, `test-web`, `test-integration`, `e2e`, `progress`
- [ ] `just` with no args prints the recipe list; `just --summary` exits 0
- [ ] `package.json` has `typecheck`, `lint`, `format:check`, `test`, `build`, `validate`
      (the last shelling to `just validate`)
- [ ] Scripts are `set -euo pipefail`, work on macOS + Linux; Windows path via `just` on
      Git Bash / pwsh is documented in a comment
- [ ] `scripts/emu-core-no-tauri.sh` greps `crates/emu-core/Cargo.toml` + `cargo tree` and
      exits non-zero if `tauri` appears
- [ ] `scripts/progress-check.mjs` (node, zero deps): validates `.agent/state.json` against
      `.agent/state.schema.json`, checks every `tasks[].id` has a matching
      `.agent/tasks/<id>-*.md` whose front-matter `status` equals the json, checks
      `currentMilestone` exists and any `done` milestone has all its `MILESTONES.md` boxes ticked
- [ ] `just progress` runs it and passes against the current repo

## Validate

```
just --summary && just progress
```

## Notes / findings
