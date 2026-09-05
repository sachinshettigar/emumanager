---
id: "0017"
title: "IPC + Create wizard screen + Dashboard wired to real Provider"
milestone: "M2"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Wire tasks `0014`-`0016` up through `tauri-specta` commands and events into the Create wizard
(`src/routes/Create.tsx`) and Dashboard (`src/routes/Dashboard.tsx`), closing out M2's DoD: create
a Play Store emulator end to end from the UI, see it reach Running, stop it.

## Context / links

- `docs/design/wireframes/` screens 1 and 2
- `src-tauri/src/commands/toolchain.rs` — the pattern to follow (thin command, typed event,
  `pub(crate)` module — see its own file-top comment on why)
- `.agent/journal/2026-09-05-session-7-dependencies-screen.md` "Gotchas" — the `job://bootstrap`
  event is meant to generalize here, not be thrown away

## Scope — files this task may touch

- `src-tauri/src/commands/emulator.rs` (new — `list_devices`, `list_images`, `create_emulator`,
  `launch_emulator`, `stop_emulator` + a generalized job-log/progress event)
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- `src/lib/ipc.ts`, `src/lib/bindings.ts` (regenerated)
- `src/routes/Create.tsx`, `src/routes/Create.test.tsx` (new)
- `src/routes/Dashboard.tsx`, `src/routes/Dashboard.test.tsx`

## Acceptance criteria

- [ ] Create wizard: device picker, image picker (installed/download-size shown, inline install),
      hardware form, review → "Create" / "Create & launch"
- [ ] Dashboard lists tracked emulators with live Stopped/Booting/Running + uptime; stop action
- [ ] `just bindings` clean; Vitest coverage for the new screens' states (loading/error/success)
- [ ] `just validate` green

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Not started.)
