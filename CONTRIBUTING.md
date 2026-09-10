# Contributing

Whether you're a human or an AI agent, the contract is **[`AGENTS.md`](AGENTS.md)**. Read it and
the docs it links before touching code.

## Setup

Prerequisites: Rust (stable), Node 20+, `pnpm`, [`just`](https://github.com/casey/just).

```bash
just setup        # install remaining toolchains + dev deps (idempotent)
just dev          # run the app in dev mode
just check-fast   # inner loop: fmt + typecheck + clippy + affected tests
just validate     # the full gate — everything CI runs; must be green before a task is done
just bindings     # regenerate src/lib/bindings.ts after changing a #[tauri::command]
```

`just package*` builds installers — see the
[README](README.md#build-an-installer-yourself) and
[`docs/playbooks/packaging.md`](docs/playbooks/packaging.md).

## Workflow

- Work is tracked as tasks in `.agent/tasks/`. Pick the next `todo` for `currentMilestone` in
  `.agent/state.json`; move it to `doing`; stay in its stated scope.
- Conventional Commits (`feat:`, `fix:`, `chore:`, …); one logical change per commit; reference
  the task id, e.g. `feat(android): create AVD (#0007)`.
- Update progress when a task is done — task file criteria, `PROGRESS.md`, `.agent/state.json`,
  and a journal entry. Follow [`docs/playbooks/update-progress.md`](docs/playbooks/update-progress.md).
- Decisions with trade-offs get an ADR ([`docs/adr/`](docs/adr/)), not a silent choice in code.

## Ground rules

- `crates/emu-core` never depends on `tauri` — domain logic stays testable with `cargo test`
  alone; `src-tauri` is glue.
- All Rust↔TS IPC goes through the generated `src/lib/bindings.ts`. Never hand-edit it.
- Never invent Android SDK behaviour — cite official docs or a captured `--help` fixture.
- No secrets or personal data in the repo. No network in unit tests.
- Keep `just validate` green on every commit to `main`.

## Map

- Plan & milestones — [`MILESTONES.md`](MILESTONES.md)
- Architecture (reference) — [`docs/architecture.md`](docs/architecture.md)
- Guided tour of the codebase — [`docs/understanding-the-codebase.md`](docs/understanding-the-codebase.md)
- Decisions — [`docs/adr/`](docs/adr/)
- Working state (tasks, journal, machine-readable progress) — [`.agent/`](.agent/)
- Recipes / playbooks — [`docs/playbooks/`](docs/playbooks/)

## Developing with AI tools

This repo is built to be developed by AI coding agents and to survive switching between them
(Claude Code, Cursor, Gemini, Antigravity, …). Every tool-specific config file is a thin stub
that points at [`AGENTS.md`](AGENTS.md) — never paste rules into it. Adding a new tool: add its
native config file as the same one-line pointer and commit it
([`docs/playbooks/bootstrap-new-harness.md`](docs/playbooks/bootstrap-new-harness.md)).
