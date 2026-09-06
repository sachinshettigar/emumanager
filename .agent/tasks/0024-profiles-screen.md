---
id: "0024"
title: "Profiles screen + export actions + Save-as-profile in the wizard"
milestone: "M4"
status: "done"
owner: "Claude Code"
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

- [x] `src/lib/ipc.ts`: `useInspectProfile` / `useApplyProfile` / `useExportProfile` / `useProfiles`
      / `useSaveProfile` / `useDeleteProfile` / `getSavedProfile`.
- [x] `src/routes/Profiles.tsx` (rewritten): a `<label>` drop zone wrapping a hidden
      `<input type="file" accept=".emuprofile,.json">`, drag-over styling, `FileReader` →
      `number[]` → `inspect_profile`. Preview: name, description, Device / Image, a **requirement
      table** (`installed` / `download N MB` per row), `ready` note or total-download figure,
      Apply / Apply & launch (streams `useEmulatorJob`; on success links to the Dashboard) +
      "Save to library". A rejected file shows the specific backend `message` verbatim.
- [x] Saved-profiles list from `list_profiles`: name + description, "Load" (fetches the JSON →
      re-inspects it into the preview) + "Delete" per row; empty state.
- [x] Emulator detail: an "Export profile" button → `export_profile(id)` → copies the JSON to the
      clipboard and shows it in a read-only `<textarea>` (chose clipboard + inline over a save
      dialog — no `tauri-plugin-dialog` dependency).
- [x] Create wizard review step: a "Save as profile" button — builds the `EmuProfile` JSON
      **client-side** from the wizard selection (parses `imageCoord`) and calls `save_profile`.
- [x] Vitest (`Profiles.test.tsx`, 4 tests): preview from a mocked `inspect_profile`, specific
      rejection message, Apply calls the command, saved list renders + Delete calls the command.
- [x] `just bindings` clean; `just check-fast` then `just validate` green (131 rust tests, 31 web).
      **M4 functionally complete** — `MILESTONES.md` M4 boxes marked; the export→wipe→import E2E is
      deferred to M6's e2e-suite line (same as M2/M3).

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### File read: `FileReader`, not `File.arrayBuffer()`

jsdom's `File` in this vitest/jsdom version has no `arrayBuffer()` method (the test threw
`file.arrayBuffer is not a function`). `readBytes` uses a `FileReader` +
`readAsArrayBuffer` promise instead — works in jsdom and the Tauri webview. The bytes go over IPC
as `number[]` (`Array.from(new Uint8Array(buf))`) — that's how `bindings.ts` types the `bytes` arg.

### Export delivery: clipboard + inline, no dialog plugin

`export_profile` returns the JSON string; the detail panel copies it to the clipboard
(`navigator.clipboard?.writeText`, guarded) and renders it in a read-only `<textarea>` for the user
to save manually. Adding `tauri-plugin-dialog` for a real "Save as…" is a follow-up — not worth a
plugin + capability entry for M4.

### "Save as profile" builds the recipe client-side

The Create wizard has `deviceId` / `imageCoord` / `name` / `ramMb` / `storageMb` but no assembled
`EmuProfile`. `saveAsProfile` parses `imageCoord` (`system-images;android-NN;<type>;<abi>`) into
`{api, type, abi}`, maps the one known tag-vs-schema drift
(`android-automotive-playstore` → `android-automotive_playstore`), builds the JSON, and calls
`save_profile` (which validates server-side via `parse_profile`). A non-phone image whose tag has
other drift would be rejected by `parse_profile` with a message — acceptable edge for M4.

### `applySaved` re-inspects

"Load" on a saved row calls `getSavedProfile(name)` → encodes the JSON to bytes → runs it back
through `inspect_profile` so the same preview + Apply path is reused (rather than a separate
"apply by name" flow).

### knip

All the `0023` + `0024` profile hooks now have a consumer (Profiles screen, detail panel, Create
wizard), so `knip` is green — this is why the hooks were held out of `0023`.
