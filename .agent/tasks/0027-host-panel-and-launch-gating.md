---
id: "0027"
title: "Dependencies host panel + launch-gating on the verdict"
milestone: "M5"
status: "todo"
owner: ""
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

The Dependencies screen grows a Host panel: verdict banner, live tiles (virtualization,
accelerator, disk, RAM), and a button per fix that calls `run_helper` and re-probes. `CannotRun`
disables "Launch" everywhere with the verdict's reason shown.

## Context / links

- Depends on task `0026` (`probe_host`, `run_helper` in `bindings.ts`)
- `src/routes/Dependencies.tsx` (has the component list + bootstrap panel), `docs/design/wireframes/`
- `src/routes/Dashboard.tsx` `EmulatorRow` + `src/routes/EmulatorDetail.tsx` Launch buttons

## Scope — files this task may touch

- `src/lib/ipc.ts` (`useHostReport`, `useRunHelper`)
- `src/routes/Dependencies.tsx` (+ a `HostPanel` section), `src/routes/Dependencies.test.tsx`
- `src/routes/Dashboard.tsx` / `src/routes/EmulatorDetail.tsx` (Launch disabled + reason when
  `verdict.kind === "cannotRun"`)
- `src/routes/Dashboard.test.tsx` / `EmulatorDetail.test.tsx` (mock `probe_host`)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] `useHostReport()` — TanStack Query for `probe_host`, a slowish refetch (30 s) since it
      shells out. `useRunHelper()` — mutation, invalidates the host report on success.
- [ ] `HostPanel` on the Dependencies screen: a verdict banner (green `canAccelerate`, amber
      `degraded` with the reason, red `cannotRun` with the reason), four tiles
      (Virtualization / Accelerator kind+status / Disk free / RAM), and, when `fixes[]` is
      non-empty, a list — each with its title + description, a "Fix it" button for
      `scriptable` fixes (→ `run_helper(fix.id)`, shows the helper's `message`, prompts a reboot
      when `needsReboot`), and manual-steps text for non-scriptable ones.
- [ ] Launch-gating: `Dashboard` row Launch and `EmulatorDetail` Launch are disabled when the host
      report's `verdict.kind === "cannotRun"`, with the reason shown next to the button. `degraded`
      launches but shows a one-line "may be slow" note.
- [ ] Graceful degradation matches `MILESTONES.md` M5 last bullet: no accelerator still lists /
      creates emulators; only launch is gated, with a clear reason + the fix.
- [ ] Vitest: `HostPanel` renders each verdict; a scriptable fix button calls `run_helper`;
      Launch is disabled under `cannotRun`.
- [ ] `just bindings` clean; `just check-fast` then `just validate` green. **M5 functionally
      complete** — mark the `MILESTONES.md` M5 boxes. The "CI runner without nested virt" DoD is
      covered by task `0025`'s `build_report` unit test; the live-CI assertion waits on M6's CI.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: how the host report is threaded to the Dashboard/detail without prop-drilling — a shared
query key; the reboot-prompt UX; whether `degraded` should also soft-warn on create.)
