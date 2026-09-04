---
id: "0013"
title: "IPC + Dependencies screen wired to real toolchain state"
milestone: "M1"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

The Dependencies screen (currently static, from task 0003) shows the real component catalog,
real installed state, and live download/bootstrap progress, driven by new typed commands.

## Context / links

- Depends on: task 0010 (catalog), task 0012 (`InstalledState`/`bootstrap`)
- Architecture: `docs/architecture.md` §4 (IPC contract, `job://progress`/`job://log`/`job://done`
  event shapes)
- Design: `docs/design/wireframes/` screen 3

## Scope — files this task may touch

- `src-tauri/src/commands.rs` (or a new `src-tauri/src/commands/toolchain.rs`) — `list_components`,
  `installed_state`, `bootstrap_toolchain` commands
- `src-tauri/src/lib.rs` (register commands + event emitters)
- `src/lib/ipc.ts` (query hooks, following the `usePing` pattern)
- `src/routes/` — the Dependencies screen component + its test
- `just bindings` diff (generated `src/lib/bindings.ts`)

## Acceptance criteria

- [ ] `list_components` returns the M1 catalog (task 0010) with `installed: bool` merged in from
      `InstalledState`
- [ ] `bootstrap_toolchain` kicks off task 0012's `bootstrap()` as a background job, emitting
      `job://progress`/`job://log`/`job://done` (or the M0-era placeholder event shape if the
      orchestrator's real event channel isn't built yet — if so, say so here and scope a follow-up)
- [ ] Dependencies screen: component list with installed/not-installed state, a working
      "Install" action wired to `bootstrap_toolchain`, live progress + log tail while running
- [ ] Frontend tests (Vitest + Testing Library) cover the installed/not-installed rendering and
      the progress-updates-while-running behavior with a mocked bindings module
- [ ] `just bindings` is clean (`git diff --exit-code src/lib/bindings.ts`)
- [ ] `just check-fast` passes

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
pnpm test
just check-fast
```

## Notes / findings

(Whether a real job-event orchestrator exists yet or this task has to add a minimal one goes
here — check task 0012's state before assuming.)
