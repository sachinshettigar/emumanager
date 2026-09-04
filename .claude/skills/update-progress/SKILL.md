---
name: update-progress
description: Update EmuManager progress tracking after finishing or pausing a task. Use when a task is done, blocked, or a work session is ending.
---

Follow `docs/playbooks/update-progress.md` in the repo. It is the source of truth; this wrapper
only exists so Claude Code surfaces it as a skill.

In short: update the task file (`.agent/tasks/`), `.agent/state.json`, `PROGRESS.md`, and add a
journal entry from `.agent/templates/journal.md`, then run `just progress` and `just validate`
before committing.
