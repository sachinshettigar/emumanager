---
id: "0032"
title: "Rename EmuManager → Emulator Studio (user-facing + bundle id)"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-08"
updated: "2026-09-08"
---

## Goal

Rename the product from **EmuManager** to **Emulator Studio** everywhere it is user-facing, plus
the bundle identifier. Internal Rust crate / npm package names stay (`emu-core`, `emu-android`,
`emu-host`, `emu-helper`, `emumanager`, `emumanager_lib`, npm `emumanager`) — they are not shown to
users and renaming them is churn with no payoff.

## Context / links

- User request (session 10, 8-item batch item #6): "Rename emumanager to Emulator Studio."
- Scope decision (user): "User-facing + bundle id" — product name, window title, About screen,
  README, docs, AND the bundle identifier `com.emumanager.desktop` → `com.emulatorstudio.desktop`.
  Rust/npm package names stay internal.
- Identifier change resets the app data-dir path (`directories` crate keys off it) and the updater
  identity — acceptable pre-1.0, no shipped users.
- GitHub repo stays `sachinshettigar/emumanager`; repo URLs in `Cargo.toml`, updater endpoint,
  README links are unchanged (the repo is not renamed).

## Scope — files this task may touch

- `src-tauri/tauri.conf.json` — `productName`, `identifier`, window `title`, `longDescription`
- `src-tauri/src/lib.rs` — crate doc comment, `.expect(...)` build-failure string
- `src-tauri/src/commands/mod.rs` — `app_info().name`
- `src-tauri/capabilities/default.json` — description prose
- `src/components/Sidebar.tsx` — brand label
- `src/routes/Dashboard.tsx` — `usePing("Emulator Studio")` + `BackendStatus` copy
- `src/routes/Dependencies.tsx` — "…install into Emulator Studio's own directory" copy
- `src/main.tsx` — `#root` missing error string
- `index.html`, `dist/index.html` — `<title>` (+ placeholder body text)
- `crates/emu-helper/src/main.rs` — clap `about` string
- `crates/emu-core/src/toolchain/bootstrap.rs` — JDK-missing user messages
- `crates/emu-core/src/model/emulator.rs` — doc comment
- `crates/emu-host/src/report.rs` — host-fix user message
- `crates/emu-core/src/lib.rs` — crate doc comment
- `justfile` — header comment
- `scripts/gen-placeholder-icons.mjs` — comment
- `README.md`, `docs/spec.md`, `docs/architecture.md`, `docs/adr/0006-require-system-jdk.md`,
  `docs/prompts/onboard.md`, `docs/prompts/review-change.md`, `docs/playbooks/release-signing.md`
- Tests: `src/routes/Dashboard.test.tsx`, `src/routes/About.test.tsx`, `src/lib/ipc.test.tsx`
- `PROGRESS.md`, `.agent/state.json`, journal

## Explicitly NOT touched

- `src-tauri/Cargo.toml` `name`/`lib.name` (`emumanager`/`emumanager_lib`), `package.json` `name`,
  `crates/*/Cargo.toml` package names, `src-tauri/src/main.rs` (`emumanager_lib::run()`)
- `scripts/progress-check.mjs` `state.project === "emumanager"` (internal id)
- `schemas/emuprofile/v1.schema.json` `$id` (`https://emumanager.app/...` — stable schema id)
- `Cargo.toml` / updater / README repository URLs (repo not renamed)
- `_typos.toml` / `clippy.toml` "EmuManager" allow-entries (still referenced in journal/history)
- `docs/design/**` wireframe files (historical design snapshot)

## Acceptance criteria

- [x] `tauri.conf.json`: `productName: "Emulator Studio"`, `identifier: "com.emulatorstudio.desktop"`,
      window `title: "Emulator Studio"`, `longDescription` updated. `bundle.active` etc unchanged.
- [x] `app_info().name == "Emulator Studio"`; About screen shows it (no code change on About.tsx —
      it renders `d.name`).
- [x] Sidebar shows "Emulator Studio"; window/tab title "Emulator Studio".
- [x] `usePing("Emulator Studio")` → backend liveness line reads "pong, Emulator Studio from vX".
- [x] No user-visible "EmuManager" string remains in `src/`, `src-tauri/src/`, `crates/*/src/`,
      `index.html`, `README.md`, `docs/*.md` (excluding the historical `PROGRESS.md` log and the
      NOT-touched list above).
- [x] `just bindings` clean; `just check-fast` then `just validate` green (rust + web tests,
      typos, markdownlint).

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- Identifier suffix stays `.desktop` — Tauri rejects an identifier ending in `.app` (recorded for
  the old id in PROGRESS.md, session 0).
