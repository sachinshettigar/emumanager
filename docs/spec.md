# EmuManager — product spec

Status: draft v1 · Scope: **B — Android emulators only, Windows / Linux / macOS**

## 1. Problem

Running Android emulators today means installing Android Studio (multi-GB IDE you don't want) or
hand-driving `sdkmanager` / `avdmanager` / `emulator` from a terminal, plus figuring out host
virtualization yourself. Sharing a working emulator setup with a teammate means writing a wiki
page. There is no small, cross-platform GUI that owns the whole lifecycle.

## 2. Users

- Mobile / web / QA engineers who need Android emulators but not the IDE.
- Test leads who want a reproducible device setup the whole team can recreate.
- Developers on Windows or Linux where the Android tooling story is rougher.

## 3. Goals

1. **Zero external setup.** From a fresh machine, the app installs every SDK component it needs
   into its own data directory. No Android Studio, no system-wide SDK, no terminal commands the
   user runs.
2. **Broad device coverage.** Every Google device profile (`avdmanager list device`) plus custom
   hardware profiles; API 21 → latest; all image types (AOSP, Google APIs, Play Store, Wear, TV,
   Automotive) and ABIs offered by the host arch.
3. **Full lifecycle in the GUI.** Create, launch (windowed or headless), stop, wipe, delete, view
   logs; state always reflects reality.
4. **Persistent tracking.** Every emulator the app created is listed with its config, source
   profile, and live state across restarts.
5. **Portable profiles.** Export an emulator as a `.emuprofile` recipe; import one on another
   machine and it resolves requirements, downloads what's missing, and creates the instance.
   **Recipes never contain SDK binaries or disk images** — only the recipe.
6. **All downloads flow through the app.** Component catalog, versions, progress, licenses — all
   in-app. No third-party updater, no copy-paste commands.

## 4. Non-goals

- iOS simulators (impossible off macOS — see ADR 0003).
- Physical device management / `scrcpy` mirroring.
- Running the emulator in the cloud or remotely (may revisit post-1.0).
- Being a general Android build tool (no Gradle, no APK building; installing an APK into a running
  emulator is in scope, building one is not).
- Full snapshot sharing across machines in v1 (recipe-only; same-arch snapshot export may come
  later).

## 5. Functional requirements

### 5.1 Dependency & SDK management (`docs/design` screen 3)

- Detect what's installed in the app's data dir: command-line tools, platform-tools, emulator,
  bundled JRE, system images (with versions + sizes).
- Resolve the component catalog (available versions) from Google's redistributable repository.
- Download with visible progress, pause/resume/cancel, checksum verification, queue.
- Accept Android SDK licenses non-interactively.
- Host readiness panel: CPU virtualization, accelerator status (KVM / WHPX-AEHD / HVF), free disk,
  RAM. Actionable guidance per OS. A one-time **elevated helper** performs the scriptable fixes
  (enable WHPX feature, add user to `kvm` group); firmware/BIOS toggles and reboots are shown as a
  checklist with live-detected status.
- Storage view: total data-dir size, breakdown, change location.

### 5.2 Create emulator (screen 2)

- Wizard: device model → system image → hardware → review.
- Device model: searchable list of Google profiles + specs; "custom hardware profile" path
  (resolution, density, RAM, storage, sensors, camera).
- System image: filter by type + ABI + API level; each row shows installed / download size; can
  trigger a download inline.
- Hardware: name, RAM, internal storage, graphics mode, snapshot/quick-boot, cold-boot, device
  frame, keyboard, network speed/latency.
- "Create" and "Create & launch". "Save as profile" writes an `.emuprofile` alongside creating.

### 5.3 Dashboard / tracking (screen 1)

- Table of every created emulator: name, device, API/image, state (running + uptime / stopped),
  source profile / imported tag, actions (launch/stop, ⋯ = wipe/delete/edit/reveal).
- Host strip: OS, accelerator, disk, RAM, re-scan.
- Selected-emulator detail: image coordinates, RAM/storage, `adb` serial, gRPC port, snapshot,
  source profile; open data folder; wipe.
- Live console: streamed emulator + adb logs for the selected instance.
- Reconcile on every launch against `avdmanager list avd` and `adb devices` / `emulator
  -list-avds` so the app never lies about state.

### 5.4 Profiles (screen 4)

- `.emuprofile` = JSON recipe: schema version, name, platform, device profile ref, image
  coordinates (`api` / `type` / `abi`), hardware params, optional seed (APKs by filename that must
  be provided separately, locale, settings). Schema: `schemas/emuprofile/v1.schema.json`.
- Export: from an existing emulator or from the create wizard.
- Import: drag-drop or file picker → parse + validate → **requirement diff** (present vs. needs
  download, with sizes) → "Download & create" → new tracked instance.
- Saved-profiles list with create-instance / export / duplicate / delete.
- Import of an iOS or otherwise unsupported profile is detected and rejected with a clear reason.

### 5.5 Cross-cutting

- All long operations are cancellable jobs that stream progress + logs to the UI; the app never
  blocks.
- Offline: the app works for launching/tracking already-installed emulators with no network;
  anything needing a download says so.
- Self-update (Tauri updater) and component-catalog refresh, both surfaced in-app.

## 6. Non-functional requirements

- **Platforms:** Windows 10/11 x64, macOS 13+ (Apple Silicon + Intel), Ubuntu 22.04+ / equivalent
  x64. ARM Linux best-effort.
- **Footprint:** app installer < 20 MB; data dir grows on demand (system images 1–3.5 GB each).
- **Cold start** to interactive dashboard < 1.5 s.
- **Security:** signed + notarized (macOS), signed (Windows). Least-privilege Tauri capabilities.
  Elevated helper is a separate audited binary invoked only on explicit user action.
- **Reliability:** killing the app never corrupts an AVD; on next start, state reconciles.
- **Accessibility:** keyboard navigable, honors OS reduced-motion and color-scheme.
- **Observability:** rotating local logs; "export diagnostics" bundle (redacted).

## 7. Success criteria for v1.0

- Fresh Windows, Linux, and macOS machine → install app → create and boot a Play Store emulator
  with **zero terminal commands** and at most one OS elevation prompt.
- Export a profile on one OS, import on another, get an equivalent running emulator.
- `just validate` green on all three OSes in CI; e2e boot test passing on Linux + Windows.

## 8. Open questions

> DECIDED (M1, task 0012): require a system JDK 17+, not a bundled JRE — see
> `docs/adr/0006-require-system-jdk.md`.

> QUESTION: default headless vs. windowed launch, and how to show the emulator window on Wayland.
> Decide by M2.

> QUESTION: do we sign `.emuprofile` files (detached signature) for provenance? Decide by M4.
