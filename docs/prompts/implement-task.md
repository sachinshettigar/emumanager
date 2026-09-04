Implement task `<NNNN>` (`.agent/tasks/<NNNN>-<slug>.md`).

Rules:
- Follow `AGENTS.md` §3 (the loop) and §6 (golden rules).
- Stay within the task's "Scope — files this task may touch". If you find you need to go outside
  it, stop and tell me — we either widen the task deliberately or split it.
- Put domain logic in `emu-core` (no `tauri` dep). `src-tauri` is glue.
- Write the tests the acceptance criteria call for. No network / real Android binaries in tests
  that `just validate` runs.
- After changing any `#[tauri::command]`, run `just bindings`. After changing a `sqlx` query, run
  `just db-prepare`.

When done:
- Run `just check-fast`, then `just validate`. Paste the result.
- Tick the acceptance criteria you actually met in the task file; set `status`.
- Do the full `docs/playbooks/update-progress.md` pass (task file, `state.json`, `PROGRESS.md`,
  journal entry).
- Propose a Conventional Commit message referencing `#<NNNN>`.

Show me the diff before committing.
