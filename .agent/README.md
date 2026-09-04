# .agent/ — tool-neutral working state

This directory is the project's working memory. It is deliberately **not** any AI tool's native
format so that development can move between Claude Code, Cursor, Gemini, Antigravity, etc. without
losing context. Everything here is committed to git.

| Path | What | Who edits |
| --- | --- | --- |
| `state.json` | Machine-readable progress: current milestone, every task + status, last validated commit. Validated by `just progress`. | any agent, on every task transition |
| `tasks/NNNN-slug.md` | One unit of work. Front-matter status + a body with goal, acceptance criteria, files in scope, and the validate command. | author creates; implementer updates status + ticks criteria |
| `journal/YYYY-MM-DD-slug.md` | Append-only session handoff notes: what changed, what's next, gotchas, decisions. | every agent at end of a work session |
| `templates/` | `task.md`, `journal.md` — copy these. | rarely |

## Task lifecycle

`todo → doing → review → done` (plus `blocked`). Only one task should be `doing` per agent at a
time. Keep the task file's `status:` and the matching entry in `state.json` in sync — `just
progress` fails the build if they drift.

## For a new agent / new tool

1. Read `AGENTS.md` (repo root).
2. Read the newest file in `journal/`.
3. Run `just progress`; open `state.json`; find the next `todo` task for `currentMilestone`.
4. Follow `AGENTS.md` §3 (the development loop).
