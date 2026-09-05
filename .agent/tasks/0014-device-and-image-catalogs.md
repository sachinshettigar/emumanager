---
id: "0014"
title: "emu-android: real device catalog + system-image catalog (parsers, no process spawn)"
milestone: "M2"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Given the bytes of two real Google-shipped sources, produce the two read models the Create wizard
needs: `Vec<DeviceProfile>` (hardware profiles) and `Vec<SystemImage>` (installable images,
`installed` flag from the local data dir). Parsing only — no `ProcessRunner`/`Downloader` calls in
this task's unit tests, matching task `0010`'s precedent.

## Context / links

- Architecture: `docs/architecture.md` §3 (`Provider::list_devices`/`list_images`), §2 ("Output
  parsing lives here, tested against captured fixtures")
- Spec: `docs/spec.md#52-create-emulator-screen-2`
- `docs/context/domain-model.md` (device profile / system image sources)
- MILESTONES.md M1's deferred line: "`sdkmanager --list` / repo XML... not needed until M2's image
  list/create flow" — this task is that flow.
- Existing pattern: `crates/emu-android/src/catalog.rs` (roxmltree, real fixture, cited source)
- Models already exist (task `0002`): `crates/emu-core/src/model/device.rs` (`DeviceProfile`,
  `Screen`, `FormFactor`), `crates/emu-core/src/model/image.rs` (`SystemImage`, `ImageCoord`,
  `ImageFilter`)

## Scope — files this task may touch

- `crates/emu-android/src/devices.rs` (new — parse `devices.xml`)
- `crates/emu-android/src/sysimg.rs` (new — parse `sys-img2-3.xml` manifests)
- `crates/emu-android/src/lib.rs` (module wiring)
- `crates/emu-android/tests/fixtures/devices.xml` (new — trimmed real capture)
- `crates/emu-android/tests/fixtures/sys-img2-3-*.xml` (new — trimmed real captures)
- `crates/emu-core/src/model/device.rs` (only if a genuinely missing field turns up while mapping
  real XML — unlikely, the model already matches)

## Acceptance criteria

- [x] `emu_android::devices::parse(xml: &[u8]) -> Result<Vec<DeviceProfile>>` reads Android's own
      `devices.xml` (see Notes for where it lives and why we parse it directly instead of shelling
      `avdmanager list device`) — screen dimensions/density/diagonal, RAM, sensors, OEM, tag→
      `FormFactor` all populated from real elements, cited
- [x] `emu_android::sysimg::parse(xml: &[u8]) -> Result<Vec<SysImgEntry>>` reads one `sys-img2-*.xml`
      manifest: every `<remotePackage path="system-images;...">` whose path parses as a plain
      (non-extension-level) `ImageCoord` becomes an entry (coord, revision, size, sha1); anything
      that doesn't parse (an `-extNN` package, present in the real feed) is skipped, not an error
- [x] `emu_android::sysimg::MANIFEST_URLS: &[(&str, &str)]` — the real, curl-verified manifest URLs
      this parser is meant to be fed (cited in Notes); a caller (task `0015`+) fetches and
      concatenates them through `Downloader`
- [x] A small static API-level → marketing-version table (cited source) backs
      `SystemImage::android_version`; unknown levels fall back to the level itself so a new API
      doesn't need a code change to show up
- [x] `installed` on both a `DeviceProfile`-adjacent lookup (n/a — devices aren't "installed") and
      `SystemImage.installed` is left `false` here — wiring it to `Fs`/`InstalledState` is task
      `0015`'s job (`ensure_image`/`create` need the same local-dir knowledge); this task returns
      the two pure catalogs only
- [x] Unit tests (fixture-driven, no network): devices parsed per form factor present in the
      fixture; sys-img entries parsed per (api × type × abi) combination in the fixtures, an
      extension-level package correctly skipped, a malformed-XML case for each parser
- [x] `cargo test -p emu-core -p emu-android --all-features` passes; `scripts/emu-core-no-tauri.sh`
      still passes
- [x] `just check-fast` and `just validate` pass

## Validate

```
cargo test -p emu-core -p emu-android --all-features
bash scripts/emu-core-no-tauri.sh
just check-fast
```

## Notes / findings

### Real-source research (before writing any parser — `AGENTS.md` §6 rule 2)

Downloaded the real `commandlinetools-mac_arm64-16111833_latest.zip` for this host and ran the
real binaries against a scratch SDK dir:

```
curl -o cmdline-tools.zip https://dl.google.com/android/repository/commandlinetools-mac_arm64-16111833_latest.zip
avdmanager list device   # real output: 15 devices, id/Name/OEM/Tag columns only
```

`avdmanager list device`'s text output has no screen/RAM/sensor data. `avdmanager` itself reads
that from XML files bundled in `lib/sdklib/sdklib.core.jar` (`unzip -l` to find them):
`com/android/sdklib/devices/{devices,nexus,wear,tv,automotive,desktop,xr}.xml`. Parsing these
directly (no process spawn) mirrors task `0010`'s choice to parse Google's real repository XML
instead of scraping `sdkmanager --list` text.

- `devices.xml` — generic/legacy profiles + "Small/Medium Phone/Tablet". No `<d:tag-id>`.
- `nexus.xml` — the actual Pixel/Nexus phones (`pixel_6` lives here, not in `devices.xml`). No
  `<d:tag-id>` either — every device here is a phone in practice.
- `wear.xml` / `tv.xml` / `automotive.xml` — **do** carry `<d:tag-id>` (`android-wear`,
  `android-tv`, `android-automotive`[`-playstore`]).
- `desktop.xml` — no tag-id, but the whole file is desktop devices.
- `xr.xml` — glasses/XR devices (the real `avdmanager list device` output's "ai_glasses_*" entries
  come from here). **Out of scope**: `FormFactor` has no XR/glasses variant and `docs/spec.md`
  never mentions one. Follow-up task if a real need shows up, same pattern as M1 deferring
  `sdkmanager --list` parsing.

Since only 3 of the 6 in-scope files tag form factor explicitly, `devices::form_factor_for` infers
Phone/Tablet/Foldable for the other 3 from id/name substrings (`"tablet"`, `"fold"`/`"rollable"`).
This is a **documented UI-classification heuristic**, not cited SDK behavior — called out clearly
in the module doc so a future reader doesn't mistake it for something `avdmanager` itself emits.

System images: confirmed by grep that a fresh real `repository2-3.xml` fetch has **zero**
`system-images` packages — unlike `cmdline-tools`/`platform-tools`/`emulator` (task `0010`), they
ship in one manifest per Google "tag" family. Curl-verified 2026-09-05 (`sysimg::MANIFEST_URLS`);
`android-automotive-playstore` has no manifest of its own — those packages are tagged inside the
`android-automotive` manifest.

### Two real data quirks found while writing the parsers

1. **RAM ships in three different units** across the real files: `nexus.xml` uses `GiB`,
   `wear.xml` uses `MiB`, `automotive.xml` uses `KiB`. All three conversion paths are exercised by
   a real fixture (`wear_tv_automotive_desktop_map_by_source`), not just the common one — this
   would have been an easy one-unit assumption to get away with using only synthetic test data.
2. **Extension-level system images don't parse as `ImageCoord`.** The real feed publishes
   `system-images;android-34-ext12;google_apis_playstore;arm64-v8a` alongside the plain
   `android-34;...` package. `ImageCoord::from_str` (task `0002`) already rejects the `-ext12`
   suffix (it expects `android-<u32>` exactly) — `sysimg::parse` reuses that existing rejection as
   the "skip this package" signal via `.filter_map` + `path.parse().ok()?`, rather than writing a
   second, separate detector for the same shape. One less thing to keep in sync.

### Fixture trimming gotcha

A naive `<d:device[^>]*>.*?</d:device>` regex used to trim `devices.xml` accidentally matches the
*root* `<d:devices ...>` open tag too (`device` is a prefix of `devices`), corrupting the trim
whenever the wanted device happened to be the first one in the source file (hit this on
`automotive.xml`'s `automotive_1024p_landscape`). Fixed the extraction script with a
`<d:device(?=[\s/>])` lookahead. Not shipped anywhere — this was a one-off trimming script, not
committed — but worth remembering if `xr.xml` or another file gets trimmed later.

### Scope note for task 0015

Locating the installed `sdklib.core.jar` and extracting `devices.xml`/`sys-img2-3.xml` bytes at
runtime (reading a real jar = a zip file) is deliberately **not** in this task — `0015` already
needs the same "where is the installed SDK" knowledge for `ensure_image`/`create`, so that's where
the extraction wiring belongs. `devices::DEVICE_XML_RESOURCES` and `sysimg::MANIFEST_URLS` are the
two lists that wiring will drive.
