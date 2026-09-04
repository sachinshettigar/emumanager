---
id: "0010"
title: "emu-android: SDK component catalog from Google's repository XML"
milestone: "M1"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Given the bytes of Google's `repository2-3.xml`, produce a typed, host-filtered list of the three
components M1 actually needs to bootstrap a working `sdkmanager` (`cmdline-tools;latest`,
`platform-tools`, `emulator`): version, download URL, size, SHA-1. No network call in this task —
parsing only, against a captured real fixture.

## Context / links

- Architecture: `docs/architecture.md` §2 ("Output parsing lives here, tested against captured
  fixtures"), §3 (`Toolchain manager` bullet under `emu-core`)
- Spec: `docs/spec.md#51-dependency--sdk-management-docsdesign-screen-3`
- Real source (cited, captured 2026-09-05): `https://dl.google.com/android/repository/repository2-3.xml`
- Existing pattern to follow: `crates/emu-core/src/model/image.rs` (`ImageCoord`
  `Display`/`FromStr`, tag strings load-bearing, fixture-cited doc comment)

## Scope — files this task may touch

- `crates/emu-core/src/model/component.rs` (new — `ComponentId`, `HostOs`, `HostArch`, `Component`)
- `crates/emu-core/src/model/mod.rs` (re-export)
- `crates/emu-android/src/catalog.rs` (new — XML parsing)
- `crates/emu-android/src/lib.rs` (module wiring)
- `crates/emu-android/Cargo.toml` (an XML parsing dep — `quick-xml` or `roxmltree`, pick one and
  say why in Notes)
- `crates/emu-android/tests/fixtures/repository2-3.xml` (new — trimmed real capture, see Notes)

## Acceptance criteria

- [x] `Component { id, version, url, size_bytes, sha1 }` in `emu-core`; `ComponentId` is a
      closed enum for the three M1 components (`CmdlineTools`, `PlatformTools`, `Emulator`) with
      `.repo_path()` returning the exact `sdkmanager` package path string
      (`"cmdline-tools;latest"`, `"platform-tools"`, `"emulator"`)
- [x] `HostOs { Linux, MacOs, Windows }` / `HostArch { X64, Arm64 }` — matches the XML's
      `<host-os>`/`<host-arch>` tag strings exactly (cited)
- [x] `emu_android::catalog::parse(xml: &[u8], os: HostOs, arch: HostArch) -> Result<Vec<Component>>`
      — for each of the 3 `ComponentId`s, finds the `<remotePackage path="...">`, picks the
      `<archive>` whose `<host-os>` (and `<host-arch>` when present — `platform-tools` has none,
      meaning "all arches") matches, and returns one `Component`. Missing package or no matching
      archive for the given os/arch → `CoreError` (not a panic)
- [x] Real fixture in `crates/emu-android/tests/fixtures/repository2-3.xml`: the *actual* XML
      captured from the URL above, trimmed to only the `<remotePackage>` elements the parser
      touches (cmdline-tools;latest, platform-tools, emulator) — well-formed XML, real bytes, not
      hand-written. `<license>`/`<channel>` elements were dropped too (parser doesn't read them
      yet — noted in the fixture's own header comment)
- [x] Unit tests (fixture-driven, no network): one `Component` resolved per (component × os ×
      arch) combination that exists in the fixture (linux/x64, macosx/x64, macosx/aarch64,
      windows — platform-tools has no arch split); a missing-package case; an
      os/arch-with-no-archive case (`windows`/`aarch64`, which the real feed doesn't publish for
      `emulator`); malformed-XML case
- [x] `cargo test -p emu-core -p emu-android --all-features` passes; `scripts/emu-core-no-tauri.sh`
      still passes (the XML parser dep goes on `emu-android`, not `emu-core`)
- [x] `just check-fast` and `just validate` pass

## Validate

```
cargo test -p emu-core -p emu-android --all-features
bash scripts/emu-core-no-tauri.sh
just check-fast
```

## Notes / findings

### XML crate

`roxmltree = "0.21"` — a DOM tree, not a streaming/serde parser. The manifest is small (even
untrimmed, ~400 KB) and the access pattern is "find one `<remotePackage>` by attribute, then read
a handful of children" a few times, not a full-document deserialize — `roxmltree`'s borrow-only
tree fits that better than `quick-xml`'s event stream or its serde derive (which would need a
struct shaped like the whole schema, most of which we don't use yet). No unsafe, no transitive
deps of its own.

### Fixture capture

```
curl -s -o /tmp/repository2-3.xml https://dl.google.com/android/repository/repository2-3.xml
```

Captured 2026-09-05, 413,485 bytes. Trimmed with a small Python script (not committed — one-off)
to the 3 `<remotePackage>` elements this parser reads, keeping every byte inside them verbatim
(same `<size>`/`<checksum>`/`<url>`/`<revision>` values a real `sdkmanager --list` run against
this exact manifest would resolve to). Result: 5,588 bytes,
`crates/emu-android/tests/fixtures/repository2-3.xml`.

### Real-file quirks a hand-written fixture would have missed

- `<url>` values are **bare filenames** (`commandlinetools-linux-16111833_latest.zip`), not full
  URLs — every consumer (including the real `sdkmanager`) resolves them against a fixed base
  (`https://dl.google.com/android/repository/`, now `catalog::BASE_URL`). Easy to assume they'd
  be absolute; they aren't.
- `<host-arch>` is **absent**, not `"any"` or similar, when an OS ships one build for every arch
  — `platform-tools` has no `<host-arch>` on any OS; `cmdline-tools` and `emulator` do have it on
  macOS specifically because Apple Silicon and Intel need different archives. The parser treats
  "no `<host-arch>` element" as "matches any arch on that OS", which only became obvious from
  seeing the asymmetry in the real data, not from a schema doc.
- `<revision>` doesn't always have all three of `<major>/<minor>/<micro>` —
  `cmdline-tools;latest` in this capture has only `<major>23</major><minor>0</minor>`, no
  `<micro>`, giving version `"23.0"` rather than a padded `"23.0.0"`.
- Real archive checksums are **SHA-1 only** (`<checksum type="sha1">`); the manifest never
  publishes SHA-256, which is what `crate::ports::Downloader::fetch` currently verifies against.
  Flagged for task 0011 to resolve (see that task's Notes placeholder) rather than decided here —
  0010 only produces the `sha1` field; it doesn't call `Downloader`.
- The real file's `emulator` package has no `windows`/`aarch64` archive (Android doesn't ship a
  native ARM64 Windows emulator build) — used as the "no matching archive" test case instead of
  inventing one, since it's a real gap in the real data.

### Not done here (by scope)

- System images (`system-images;...`) are **not** parsed by this task — they come from a
  different Google feed (per-vendor `sys-img2-*.xml`), are M2 scope (`ensure_image`), and already
  have their own `ImageCoord`/`SystemImage` types from task 0002.
- No network call anywhere in the crate or its tests — `parse()` takes bytes; task 0011 owns
  fetching them for real.
