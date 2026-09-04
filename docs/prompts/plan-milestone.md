Break milestone `<MX>` (`MILESTONES.md`) into `.agent/tasks/` entries.

For each task:
- Use `.agent/templates/task.md`. Number sequentially after the highest existing task id.
- One coherent unit of work — a few hours, not days. If it's bigger, split it.
- Fill "Scope — files this task may touch" concretely (paths, even if the files don't exist yet).
- Acceptance criteria must be checkable and map back to the milestone's DoD.
- "Validate" is the exact command(s) that prove the task done.
- Set `milestone: "<MX>"`, `status: "todo"`.

Also:
- Order them (note dependencies) and propose a sequence.
- Add every task to `.agent/state.json` `tasks[]`.
- Flag any DoD item that needs an ADR first, or a spec open question that must be resolved.
- Run `just progress` to confirm state.json and the task files agree.

Show me the list of tasks (id + title + one-line goal + deps) before writing the files.
