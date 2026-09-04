You are working on EmuManager. Before writing any code:

1. Read `AGENTS.md` at the repo root, fully, plus `docs/spec.md` and `docs/architecture.md`.
2. Read the newest file in `.agent/journal/`.
3. Run `just progress` and open `.agent/state.json`. Run `git log --oneline -15`.
4. Run `just setup` then `just validate` to confirm the environment. If `just validate` is red,
   check `PROGRESS.md` / the latest journal for why.

Then tell me:
- the current milestone and the next `todo` task,
- anything the journal flags as a gotcha,
- whether the environment is green.

Do not start implementing until I confirm the task.
