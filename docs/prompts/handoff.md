We're ending this session. Do a clean handoff so any tool can resume:

1. If a task is mid-flight, make sure the code is in a coherent state (compiles; tests you added
   pass or are marked `#[ignore]`/`.skip` with a note). If it isn't, either finish the smallest
   coherent slice or revert to the last green point.
2. Run `just validate`. Record the result and the commit hash.
3. Do the full `docs/playbooks/update-progress.md` pass:
   - task file(s): tick real criteria, set `status` (`doing`→`review`/`done`/`blocked`),
   - `.agent/state.json`: statuses match, `lastValidatedCommit` set if green,
   - `PROGRESS.md`: Current state block + a Log entry,
   - new journal entry from `.agent/templates/journal.md` — the **Next action** line must be
     startable from cold, and list any gotchas.
4. Run `just progress` — must pass.
5. Commit everything (Conventional Commits). Don't leave uncommitted changes.

Then give me a 3-line summary: what got done, what's next, anything risky.
