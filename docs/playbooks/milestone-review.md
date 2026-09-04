# Playbook: close out a milestone

Run this when you think a milestone (`MILESTONES.md`) is done.

## Checklist

1. **Every DoD box ticked** in `MILESTONES.md` for the milestone — and each is *actually* true,
   not aspirational. Walk them one by one.
2. **`just validate` green** on the current HEAD, on your OS. Note the commit hash.
3. **CI green** for that commit on all three OSes (`ci.yml`) and `schema.yml`.
4. **Coverage** meets the milestone's gate (Rust + web). If you bumped the gate, the numbers
   clear the new bar.
5. **E2E / integration** for the milestone's DoD demo has run and passed (Linux + Windows for
   full e2e; macOS via the manual checklist from M6 onward). Paste the run link/output into the
   milestone-review journal entry.
6. **Docs current:** `docs/architecture.md`, `docs/context/`, `docs/spec.md` open questions
   resolved or explicitly carried forward. New ADRs written for any decision made along the way.
7. **No `blocked` tasks** for this milestone left dangling — each is `done`, or explicitly moved
   to a later milestone with a note.
8. **Demo it.** Record (or write) a short "here's the milestone working" walkthrough. For M2+
   this is a screen capture of the app doing the DoD flow.

## Then

- `.agent/state.json`: set the milestone `status: "done"`, bump `currentMilestone`, set
  `lastValidatedCommit`, update `coverageGate`.
- `MILESTONES.md`: the milestone header stays; boxes stay ticked.
- `PROGRESS.md`: tick the milestone in the checklist; add a Log entry.
- Journal: a `milestone-MX-review` entry with the CI links, coverage numbers, demo, and anything
  deferred.
- Tag: `git tag mX-done` (annotate with the summary).
- If this repo uses branch protection, confirm `ci.yml` is a required check before opening the
  next milestone's work.

## If it's not actually done

Don't fudge the boxes. List what's missing in the journal, keep `currentMilestone` where it is,
and create/adjust tasks for the gaps.
