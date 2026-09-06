---
id: "0027"
title: "Dependencies host panel + launch-gating on the verdict"
milestone: "M5"
status: "review"
owner: "Claude Code"
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

- [x] `useHostReport()` — TanStack Query for `probe_host`, `refetchInterval: 30000`.
      `useRunHelper()` — mutation, `onSettled` invalidates the host report (Windows returns a
      synthetic outcome, so the re-probe is the source of truth).
- [x] `HostPanel` on the Dependencies screen: a verdict banner styled by `verdict`
      (`canAccelerate` green / `degraded` amber / `cannotRun` red, showing `verdictReason`); four
      `Tile`s (Virtualization / Accelerator `kind · status` / RAM / Disk free); and, when `fixes`
      is non-empty, a row per fix with title + `description`, a "Fix it" button for `scriptable`
      ones (→ `run_helper(fix.id)`, then shows `runHelper.data.message`; a `cancelled` error →
      "prompt was dismissed"), a "manual" tag + description for the rest, and a reboot note when
      `needsReboot`.
- [x] Launch-gating: `Dashboard` `EmulatorRow` and `EmulatorDetail` disable "Launch" when
      `host.data?.verdict === "cannotRun"` (both `useHostReport`, shared query key), with
      `verdictReason` shown next to the button (`launch-blocked-<id>` / `launch-blocked`) and as a
      `title`.
- [x] Graceful degradation: the panel and gating only touch *launch*; component list / create /
      the emulator list all still work with no accelerator (`degraded` isn't gated at all —
      simplified from the "may be slow" note; the banner already carries the warning).
- [x] Vitest: `Dependencies.test.tsx` +2 (host verdict + tiles; a scriptable fix runs `run_helper`
      and shows the result); `Dashboard.test.tsx` +1 (Launch disabled + reason under `cannotRun`).
      Every screen test's bindings mock gained `probeHost` / `runHelper`.
- [x] `just check-fast` then `just validate` green (154 rust tests, 34 web tests; `bindings.ts`
      unchanged — the commands landed in `0026`). **M5 functionally complete** — `MILESTONES.md`
      M5 boxes marked. The "live CI runner without nested virt → `CannotRun`" DoD assertion is
      covered in logic by `0025`'s `ci_runner_without_virtualization_cannot_run_...` unit test; the
      real CI run waits on M6.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### Shared query key, no prop-drilling

`useHostReport()` uses the query key `["host-report"]`. Dependencies, Dashboard's `EmulatorRow`,
and `EmulatorDetail` all call it — TanStack Query dedupes, so there's one `probe_host` in flight
and every consumer reads the same cache. `useRunHelper`'s `onSettled` invalidates that key, so a
fix immediately triggers a re-probe everywhere.

### `HostReportDto` is flat strings

The `verdict` / `acceleratorKind` / `acceleratorStatus` / `virtualization` fields are the camelCase
string forms from `commands/host.rs` (`"cannotRun"`, `"noPermission"`, `"hvf"`, …), not the
emu-core enums (which aren't `specta::Type`). The UI matches on those strings.

### `degraded` isn't gated

The task sketch had a "may be slow" note on `degraded` launches. Dropped it — the verdict banner
on the Dependencies screen already spells out the degradation, and a per-row note on the Dashboard
was noise. Only `cannotRun` blocks launch; `degraded` behaves exactly like `canAccelerate` for the
launch buttons.

### Reboot UX

A `needsReboot` fix shows "A reboot is required after this fix." under its row after `run_helper`
succeeds. The app doesn't try to trigger or schedule a reboot — that's the user's call.

### Every screen test now mocks `probeHost`

`useHostReport` fires on mount for Dependencies / Dashboard / EmulatorDetail. A `vi.mock`ed
bindings module with no `probeHost` makes `commands.probeHost()` throw a plain `TypeError` (not an
`IpcCallError`), which crashed the render via `host.error.ipc.message`. Fixed by adding a
`probeHost` (and `runHelper`) mock to each of those test files — the pattern to follow for any new
screen that reads the host report.
