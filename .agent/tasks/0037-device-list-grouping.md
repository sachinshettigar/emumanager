---
id: "0037"
title: "Group the device list by form factor + richer device rows"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

The Create wizard's device step was one long flat list. Group it by device type and make each
row say more. Sixth of the 8-item feature batch (item #8: "Device lists are not good — make it
more meaningful, group by device type. Provide customization options similar to Android Studio").

## Context / links

- User request (session 10): "Device lists are not good. Can we make it more meaningful - group
  by device type. Provide customization options similar to android studio."
- `src/routes/Create.tsx` `DeviceStep` — flat `useMemo` filter + a single `<ul>`.
- `DeviceInfo` already carries `formFactor` (`phone`/`tablet`/`foldable`/`wear`/`tv`/`automotive`/
  `desktop`) and, since task 0035, `skin`.

## Scope — files this task may touch

- `src/routes/Create.tsx` — `DeviceStep` grouped into collapsible `<details>` sections by form
  factor; a `DeviceRow` component; a "frame" badge when the device has a skin
- `src/routes/Create.test.tsx` — a grouping test
- `MILESTONES.md` note (custom hardware-profile editor stays M7), `PROGRESS.md`,
  `.agent/state.json`, journal

## Acceptance criteria

- [x] `DeviceStep` renders one `<details data-testid="device-group-<factor>">` per non-empty form
      factor, in a fixed order (Phones, Tablets, Foldables, Wear OS, Android TV, Automotive,
      Desktop, Other), each `<summary>` showing the label + count.
- [x] Groups are collapsed by default; a group opens automatically while a search is active, when
      it holds the selected device, or when it's the only group.
- [x] Searching filters across all groups and drops groups with no matches; "No devices match" is
      shown when nothing matches.
- [x] Each row shows name (+ a "frame" badge when `skin` is set), OEM, resolution, dpi, RAM.
      `data-testid="device-<id>"` unchanged so the wizard flow tests still pass.
- [x] Vitest: "groups devices by form factor" (phone + wear fixture → two groups with counts; a
      search narrows to one). 43 web tests.
- [x] `just validate` green (no bindings change — pure frontend).

## Validate

```
just validate
```

## Notes / findings

- **Customization stays the existing hardware step.** RAM / storage / graphics / device-frame are
  already editable in the wizard's Hardware step and the detail panel (tasks 0021, 0035). A full
  Android-Studio-style *custom device profile* editor (screen size / resolution / density / cores,
  saved as a reusable profile) is a bigger feature and is already on `MILESTONES.md` for M7
  ("custom hardware profile editor") — not pulled forward here. This task delivers the concrete
  half of the request ("group by device type", "more meaningful").
- Native `<details>`/`<summary>` for the collapsibles — no JS state, keyboard-accessible, and
  jsdom keeps collapsed children in the DOM so the existing click-through tests are unaffected.
