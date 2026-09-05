---
id: "0015"
title: "emu-android: AndroidProvider — ensure_image (download+install) + create (avdmanager create avd)"
milestone: "M2"
status: "todo"
owner: ""
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

The first real `Provider` implementation. `ensure_image` downloads+extracts a chosen system image
into the managed SDK dir (reusing the `Downloader`/zip-extraction pattern from
`crates/emu-core/src/toolchain/bootstrap.rs`) and accepts its license; `create` drives
`avdmanager create avd` via `ProcessRunner`, parses the result, and inserts a minimal row into the
existing `emulators` table (`migrations/0001_init.sql` — the full M3 schema is not needed for this;
`Registry`'s current columns are enough to remember `id`/`avd_name`/`display_name`).

## Context / links

- Architecture: `docs/architecture.md` §3 (`Provider::ensure_image`/`create`), §5 ("Create +
  launch" flow steps 2-4)
- Depends on task `0014`'s catalogs (image coord → download URL/sha1) and task `0012`'s
  `zip`-extraction pattern (**re-read its `Send`-safety gotcha before touching archive code** —
  see `.agent/journal/2026-09-05-session-7-dependencies-screen.md`)
- `crates/emu-core/src/provider.rs` — the trait to implement
- `crates/emu-core/src/registry/open.rs` — current `Registry` surface

## Scope — files this task may touch

- `crates/emu-android/src/provider.rs` (new — `AndroidProvider` struct + `ensure_image`/`create`)
- `crates/emu-android/src/lib.rs`
- `crates/emu-core/src/registry/open.rs` (an `insert_emulator`/`get_emulator` method or two, if the
  existing surface doesn't have room)
- Fixtures for `avdmanager create avd` success/failure text output (real capture, cited)

## Acceptance criteria

- [ ] `ensure_image` is a no-op (fast success) when already installed; otherwise downloads via
      `Downloader`, verifies SHA-1 (see task `0012`'s `Downloader::fetch` only supports SHA-256 —
      same workaround: verify at the call site), extracts, accepts the image's license
- [ ] `create` builds the right `avdmanager create avd -n <name> -k <coord> --device <id>` argv
      (real flags, cited `avdmanager create avd --help`), runs it non-interactively (auto-accept
      the "custom hardware profile" prompt the real tool asks), parses success/failure from exit
      code + stderr, and returns a new `EmulatorId`
- [ ] A tracked row lands in the `emulators` table on success
- [ ] Unit tests with fake `ProcessRunner`/`Downloader`/`Fs`; a real captured `--help` / sample
      output fixture backs the argv and parsing
- [ ] `just check-fast` passes
- [ ] Docs updated if behavior/interface changed

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just check-fast
```

## Notes / findings

(Not started.)
