---
id: "0024"
title: "Profiles screen + export actions + Save-as-profile in the wizard"
milestone: "M4"
status: "todo"
owner: ""
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

The M4 UI: a Profiles screen with a drop zone, an import preview (requirement diff + sizes), a
saved-profiles list with apply/delete; an "Export profile" action on the emulator detail panel;
and "Save as profile" in the Create wizard's review step.

## Context / links

- Depends on task `0023` (`inspect_profile`, `apply_profile`, `export_profile`, `save_profile`,
  `list_profiles`, `delete_profile` in `bindings.ts`)
- `src/routes/Profiles.tsx` (current static placeholder), `docs/design/wireframes/` screen 4
- `src/routes/Create.tsx` review step, `src/routes/EmulatorDetail.tsx` actions row

## Scope — files this task may touch

- `src/lib/ipc.ts` (the `0023` hooks: `useInspectProfile` / `useApplyProfile` / `useExportProfile`
  / `useProfiles` / `useSaveProfile` / `useDeleteProfile`)
- `src/routes/Profiles.tsx` (rewritten), `src/routes/Profiles.test.tsx` (new)
- `src/routes/EmulatorDetail.tsx` (+ "Export profile" → download / reveal the written file, or a
  copy-JSON fallback), `src/routes/Create.tsx` ("Save as profile" in review)
- `src/routes/Dashboard.test.tsx` / `EmulatorDetail.test.tsx` (mock the new commands if referenced)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] Profiles screen: a real drop zone (`<input type="file" accept=".emuprofile,.json">` +
      drag-over styling) → reads the file → `inspect_profile` → renders the preview: name,
      description, device, image, and a **requirement table** (each row `Present` / `needs N MB`),
      a total-download figure, and an "Apply" button (disabled while a job runs). A rejected file
      shows the specific backend message.
- [ ] Saved-profiles list from `list_profiles`: name + summary, "Apply" and "Delete" per row;
      empty state.
- [ ] The apply flow streams the same job-log panel the Create wizard uses (`useEmulatorJob`), and
      on success links to the new emulator on the Dashboard.
- [ ] Emulator detail: an "Export profile" button → `export_profile(id)` → offer the JSON (write to
      a file the user picks, or — simplest, note the choice — copy to clipboard + show it).
- [ ] Create wizard review step: a "Save as profile" button → `save_profile` of the wizard's
      current selection as an `EmuProfile`.
- [ ] Vitest: Profiles screen renders a preview from a mocked `inspect_profile`, shows a rejection
      message, applies a profile; saved list renders + delete calls the command.
- [ ] `just bindings` clean; `just check-fast` then `just validate` green. **M4 functionally
      complete** — mark the `MILESTONES.md` M4 boxes; the true export→wipe→import E2E is deferred to
      M6's e2e-suite line (same as M2/M3).

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

(Fill in: how "export" delivers the file without a Tauri save-dialog plugin — clipboard + a
`reveal_path` of a written file, or add the dialog plugin; the drop-zone file-read approach under
jsdom for the test.)
