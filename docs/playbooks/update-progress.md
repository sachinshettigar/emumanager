# Playbook: update progress

Do this every time you finish a task, pause one, or end a work session. It is how the next agent
(possibly in a different tool) continues without you.

## 1. The task file (`.agent/tasks/NNNN-*.md`)

- Tick the acceptance-criteria checkboxes you actually satisfied. Do **not** tick ones you didn't.
- Set front-matter `status`:
  - `review` — code done, `just validate` green, but wants a second pair of eyes / not yet merged.
  - `done` — merged and verifiably complete.
  - `blocked` — add a `## Notes` paragraph: what blocks it, what would unblock it.
- Set `owner` to your tool name, bump `updated`.
- Add findings to `## Notes / findings` (captured CLI output, a decision deferred to an ADR, a
  surprise). Future-you will want these.

## 2. `.agent/state.json`

- Update the matching `tasks[]` entry's `status` to the same value as the task file.
- If you ran `just validate` green on a commit, set `lastValidatedCommit` to that hash.
- If a milestone's every `MILESTONES.md` box is now ticked and `just validate` is green, set that
  milestone's `status` to `done` and bump `currentMilestone`. Otherwise leave it `in_progress`.
- Bump `updated`.

## 3. `PROGRESS.md`

- Update the **Current state** block: milestone, phase, `lastValidatedCommit`, next action.
- Tick the milestone checklist if a milestone closed.
- Prepend a dated entry to **Log** (2–6 lines: what changed, why, what's next).

## 4. Journal entry

- `cp .agent/templates/journal.md .agent/journal/$(date +%F)-<slug>.md` and fill every section.
- The **Next action** line must be concrete enough to start from cold.

## 5. Check + commit

```
just progress        # fails if task files and state.json disagree
just validate        # unless you're intentionally leaving it red (say so everywhere)
git add -A && git commit -m "type(scope): summary (#NNNN)"
```

`just progress` is the guardrail — if it fails, your state files are inconsistent; fix them before
committing.
