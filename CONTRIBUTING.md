# Contributing

Whether you're a human or an AI agent, the contract is **[`AGENTS.md`](AGENTS.md)**. Read it and
the docs it links first.

Short version:

- Work is tracked as tasks in `.agent/tasks/`. Pick the next `todo` for `currentMilestone` in
  `.agent/state.json`.
- `just check-fast` in the inner loop; `just validate` must be green before a task is done.
- Conventional Commits; one logical change per commit; reference the task id.
- Decisions with trade-offs get an ADR (`docs/adr/`). Progress updates follow
  `docs/playbooks/update-progress.md`.
- `crates/emu-core` never depends on `tauri`. IPC only through generated `src/lib/bindings.ts`.
  No secrets or personal data. No network in unit tests.
