# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **`0030` (E2E harness) done** (session 10) — with `0029`, the last open M6 items that aren't
  blocked on CI billing. New self-contained `e2e/` npm project (own `package.json` /
  `node_modules`, outside the pnpm workspace, so it never affects `just validate`):
  `wdio.conf.ts` (spawns/kills `tauri-driver`), one smoke spec (`e2e/specs/smoke.e2e.ts` — app
  launches, Dashboard + sidebar render, the typed IPC seam reaches a `pong`), a `tsconfig` and a
  README. `just e2e` → `scripts/e2e.sh`: on Linux/Windows it builds the debug app and runs
  `wdio`; on macOS it prints a pointer to the new `docs/playbooks/macos-e2e-checklist.md` (the
  manual M2-DoD flow) and exits 0. An `e2e` job was added to `ci.yml` (ubuntu + windows,
  `cargo install tauri-driver`); the nightly's placeholder e2e step is now the real thing. None
  of the CI runs until the GitHub Actions billing block is lifted — that unblocks M0's `ci.yml`,
  the M2–M5 "real boot" DoD lines, and M6's `release.yml` all at once.
- **`0029` (logging + diagnostics) done** (session 10): `src-tauri` finally has real logging —
  `src-tauri/src/logging.rs` sets up a `tracing` subscriber with a daily-rotating file under
  `<data_dir>/logs/emulator-studio.log` (plus stderr in debug), `EnvFilter` from `RUST_LOG` else
  `info`. The startup `reconcile()` and the exit child-reaper now log their outcome instead of
  `let _ = …`. New `export_diagnostics()` command bundles the log tail + a fresh host report +
  `{app,tauri,os,arch}` versions + a hand-built (notes/tags-omitted) emulators table into
  `<data_dir>/diagnostics-<ts>.zip`, with home-path → `~` redaction (non-path secrets are a
  documented non-goal — the app's own logs are authored not to contain any). "Export diagnostics"
  button at the bottom of the Dependencies screen → reveals the zip.
- **Learning document delivered** (session 10, end of the 8-item batch):
  `docs/understanding-the-codebase.md` — an orientation guide (complements the
  `docs/architecture.md` reference): the five layers + the no-`tauri`-in-`emu-core` rule, the
  ports/adapters idea, the typed IPC + event seam, the registry, a full "click Launch" trace, the
  add-a-feature recipe, a where-things-live map, the dev-loop gotchas, and the load-bearing
  conventions. Linked from `README.md` and `AGENTS.md`.
- **Phase (8-item feature batch, session 10):** user asked for seven things at once — live device
  telemetry (storage/network/logs), a logcat-style live+filterable viewer, real download/run
  progress %, proper device skins, export-profile parity, a rename, and grouped/customizable
  device lists. Scoped into `0032`–`0038`. **`0038` (device inspector) done — the batch is
  complete.** The emulator detail panel gained a "Device" section: a facts strip (model, Android
  version + API, battery %, `/data` free/total — from defensively-parsed `adb shell` one-shots,
  polled every 5 s) and a live `adb logcat -v threadtime` viewer with Android-Studio-style
  filters — minimum level (V/D/I/W/E), tag, free text — plus pause (freezes a snapshot), clear,
  and a shown/total line counter. Backend: `AndroidProvider::{logcat_start,logcat_stop,
  device_facts}` + a `logcats` child map reaped on shutdown; a `device://log` event streamed by a
  background drain task. **`0037` (device grouping) done:** the Create wizard's
  device step was one flat list; it's now grouped into collapsible sections by form factor
  (Phones / Tablets / Foldables / Wear OS / Android TV / Automotive / Desktop) with per-group
  counts, collapsed by default and auto-opening on search or selection. Each row shows OEM /
  resolution / dpi / RAM and a "frame" badge. The full custom-device-profile editor stays M7.
  **`0036` (export parity) done:** export was
  clipboard-only and buried on the detail panel; now `export_profile_to_file` writes a real
  `<data_dir>/exports/<avd>.emuprofile` and returns its path. The detail panel offers "Copy
  profile" and "Save as .emuprofile" (→ path + "Show in folder"), and every Dashboard row has an
  "Export" button — the mirror of the Profiles screen's drag-in import (no save-as dialog: the
  dialog plugin isn't wired, deliberately). **`0035` (device skins) done:** `Hardware.device_frame`
  — stored but read by nothing until now — is load-bearing. `DeviceProfile` carries the `<d:skin>`
  name; `AndroidProvider::launch` passes `-skin <name> -skindir <sdk>/skins` when the frame is
  wanted **and** the skin is actually installed under `<sdk>/skins/` (a cmdline-tools-only SDK
  has none — the launch log then says so instead of the emulator printing a scary warning). A
  "Show device frame" checkbox in the Create wizard and the detail-panel hardware form.
  **`0034` (progress) done:** `bootstrap` now streams
  `sdkmanager`'s own output line by line (`ProcessRunner::spawn`) instead of dumping it at the
  end — the ~900 MB `emulator` download no longer looks frozen. No output-format parsing (no
  fixture; AGENTS §6.2): the `sdkmanager` phase shows an indeterminate pulse bar, while the
  `cmdline-tools` archive download — which `NativeDownloader` already measures — shows a real
  percentage. `RunProgress` gained that bar; Dashboard/Detail show a pulsing "booting…" dot.
  **`0033` (uninstall) done:** the Dependencies screen's per-row Install now has a mirror — an "Uninstall" button (→ "Confirm remove" / "Cancel") on any
  installed, app-managed component that isn't the command-line tools. New
  `emu_core::toolchain::uninstall` module runs `sdkmanager --uninstall <pkg> --sdk_root=…`;
  it refuses to touch a component that came from a system SDK (Android Studio, `ANDROID_HOME`) —
  both client-side (no button) and server-side (`Unsupported`). `uninstall_component` command,
  `useUninstallComponent` hook. 6 core + 2 web tests. **`0032` (rename) done:** the product is now
  **Emulator Studio** everywhere user-facing — `tauri.conf.json` `productName` + window title,
  `app_info().name`, the sidebar brand, the window/tab `<title>`, backend liveness ping, all
  user-visible copy and error strings in `crates/*`, and README + `docs/*`. Bundle identifier
  `com.emumanager.desktop` → `com.emulatorstudio.desktop` (resets the app data-dir path and the
  updater identity — fine pre-1.0, no shipped users). Internal names are unchanged: Rust crates
  (`emu-core`/`emu-android`/`emu-host`/`emu-helper`, `emumanager`/`emumanager_lib`), the npm
  package, the GitHub repo and every URL pointing at it, the `.emuprofile` schema `$id`. No
  bindings change; `just validate` green. Next: `0033` (uninstall SDK components).
- **Milestone:** M6 — Cross-platform hardening & packaging. **Scope narrowed by ADR 0007**: v1
  ships **unsigned** installers (project owner's call — no Apple/Windows certs; the Tauri updater
  keeps its own Ed25519 signature). Scoped into `0028`–`0030`; `0028` (packaging + `release.yml`)
  **done** (in review). M5 (and M4/M3/M2) stay `in_progress` on lines that need a green CI run.
- **Phase:** `0028` landed the packaging path. `tauri.conf.json` now bundles (`active: true`,
  `targets: all`, updater artifacts) with real `.icns`/`.ico` icons and a generated Ed25519
  updater pubkey; `tauri-plugin-updater` is registered; `capabilities/default.json` is
  least-privilege (`core:default` + `updater:default`). `.github/workflows/release.yml` builds
  unsigned installers + signed updater artifacts on a `v*` tag (macOS arm64+x64 / Linux /
  Windows), `actionlint`-clean. `just package` verified locally — a 6.8 MB `.dmg` came out.
  ADR 0007 + `docs/playbooks/release-signing.md` document the unsigned decision and the key flow.
  `0031` then added, on user request: **each SDK component installs on its own** (per-row Install
  button + `install_component` command), a Dashboard **first-run onboarding checklist**, and an
  **About** screen (version, licence, per-feature summary). Next: `0029` (rotating logs +
  `export_diagnostics`), then `0030` (E2E harness skeleton).
- **Phase:** M4 done end to end. `0024`: `src/routes/Profiles.tsx` rewritten — a `FileReader`
  drop zone → `inspect_profile` → a requirement table + Apply / Apply & launch (streams
  `useEmulatorJob`) + Save-to-library; a saved-profiles list with Load / Delete. Emulator detail
  gets "Export profile" (clipboard + a read-only `<textarea>`); the Create wizard's review step
  gets "Save as profile" (builds the `EmuProfile` JSON client-side). `ipc.ts` got the seven
  profile hooks. Next: scope M5 — Host readiness & elevated helper.
- **Phase:** M3 done end to end. `0018`: full schema + typed `Registry` API. `0019`: `reconcile()`,
  `delete()`, kill-safety, and the DoD property test (real `avdmanager list avd` fixture captured
  from a real SDK). `0020`: the `AndroidProvider` is now one shared `tauri::State`
  (`src-tauri/src/provider_state.rs`, `OnceCell`-built on first command) — its spawned-child map
  persists (`stop_emulator` reaches the force-kill fallback) and a `RunEvent::ExitRequested`
  handler reaps every child on quit (**the M2 "app exit orphans an emulator" gap is closed**);
  `reconcile()` runs once at startup; 7 new lifecycle commands. `0021`: the `/emulator/:id` detail
  panel (`src/routes/EmulatorDetail.tsx`) — config + live state, inline rename, RAM/storage/graphics
  form, Wipe/Delete (with confirm) / Launch-or-Stop, "Open folder" (`reveal_path`) — and a
  per-emulator log console: `launch` tees its output stream to `<data_dir>/logs/<avd>.log`,
  `emulator_log_tail` reads the history tail, live lines come off `job://emulator`. 17 commands total.
  Next: verify M3 tasks → `done`, flip M3, start M4 (Profiles: export / import / recreate).
- **M2 recap:** tasks `0014`–`0017` all **done**; M2 stays `in_progress` only on its one DoD line
  (a real `tauri-driver` E2E boot), deferred to M6's e2e-suite work.
- **M2 task 0017 (IPC + Create wizard + Dashboard) done:** `src-tauri/src/commands/emulator.rs` —
  six `tauri-specta` commands (`list_devices` / `list_images` / `list_emulators` /
  `create_emulator` / `launch_emulator` / `stop_emulator`) + one `EmulatorJob` event
  (`job://emulator`, mirroring `toolchain::BootstrapProgress`). `AndroidProvider` is built
  per-command (like `toolchain.rs` builds its ports per call) — documented consequence: the
  child-handle map from `0016` doesn't persist, so `stop` uses the graceful `adb emu kill` path
  without a held handle (fine for M2; a shared/managed provider is M3). `list_devices`/`list_images`
  are done in the **shell**, not on the `Provider` trait (its signatures can't take manifest bytes
  or a `Downloader`, and `reqwest` in `emu-android` would break "network behind a port"):
  `devices::parse_from_jar` (new pure fn + `zip` dep on `emu-android`) on the installed `sdklib`
  jar read from disk, `sysimg::parse` on the six `sys-img2-3.xml` manifests fetched concurrently
  with `futures_util::try_join_all` — exactly `toolchain::resolve_catalog`'s pattern. New
  `AndroidProvider` surface: `sdk_root` (now `pub`), `is_image_installed`, `tracked_states()` (a
  lightweight read-only Dashboard view — registry rows + live `RunState` from `adb`, **not**
  `reconcile()`); plus `Registry::list_emulators()`. `Create.tsx` rewritten as a 4-step wizard,
  `Dashboard.tsx` rewritten to list emulators (4 s poll) with per-row Launch/Stop; new hooks in
  `ipc.ts`. 22 web tests (+9), 3 new Rust unit tests, 114 Rust tests total. Wizard scope trims
  (hardware form is name/RAM/storage only, inline image download folded into "Create", "Save as
  profile" is M4, uptime needs the M3 schema) are documented in the task file and `MILESTONES.md`.
- **M2 task 0014 (device catalog + system-image catalog) done:** two new pure-parser modules in
  `emu-android` — `devices::parse` reads Android's own hardware-profile XML files (`devices.xml`,
  `nexus.xml`, `wear.xml`, `tv.xml`, `automotive.xml`, `desktop.xml`, all shipped inside
  `cmdline-tools`' `sdklib.core.jar`) directly, since `avdmanager list device`'s plain-text output
  doesn't carry the screen/RAM/sensor data `DeviceProfile` needs; `sysimg::parse` reads Google's
  per-tag `sys-img2-3.xml` manifests (system images live outside `repository2-3.xml`, one manifest
  per Google "tag" family — 6 real, curl-verified URLs). Both real sources were captured fresh
  (2026-09-05) via a real `cmdline-tools` download + real `sys-img2-3.xml` fetches, trimmed to real,
  verbatim bytes in `crates/emu-android/tests/fixtures/`. 19 new fixture-driven unit tests, no
  network in `just validate`. See the task file's Notes for the two data quirks this surfaced:
  RAM ships in three different units (GiB/MiB/KiB) across the real files, and an extension-level
  system-image package (`android-34-ext12;...`) doesn't round-trip through the existing
  `ImageCoord::from_str` — reused as the skip signal rather than adding separate detection logic.
- **M2 task 0015 (`AndroidProvider` — `ensure_image` + `create`) done:** the first real `Provider`
  implementation. Real research **changed the task's own plan mid-flight**: rather than
  downloading a system image via `Downloader` and extracting it ourselves (the original scope), a
  real install (`yes | sdkmanager "system-images;android-34;default;x86_64"` against a real
  scratch SDK, with a real 720 MB image downloaded for this purpose) proved `sdkmanager` already
  fetches/verifies/unpacks its own packages correctly — same reasoning
  `crates/emu-core/src/toolchain/bootstrap.rs` already uses for `platform-tools`/`emulator` — so
  `ensure_image` ended up `ProcessRunner`-only, no `Downloader`/zip code at all. Real install
  directory confirmed: `sdkmanager` mirrors the package path directly, `;` → `/`
  (`system-images/android-34/default/x86_64/`), and the marker it checks (`source.properties`) is
  the same file every real `sdkmanager` package writes. `create()` drives a real
  `avdmanager create avd -n/-k/-d`; there's no `--help` for this subcommand, so the real flags came
  from a live "unknown flag" usage dump, and its three real failure texts (duplicate name, unknown
  device, invalid package) back a specific `CoreError` for each — `--force` is deliberately never
  passed (a name collision should error, not silently overwrite). Added `EmulatorId::generate()`
  (a real `ulid` dependency) since `Emulator`'s own doc comment already promised "a ULID string in
  practice" but nothing generated one; `Registry` gained `insert_emulator`/`get_emulator` against
  the existing M1-era `emulators` table (no new migration — full schema is M3). 17 new tests (8
  `emu-core`, 9 `emu-android`), all fixture/fake-driven; `just validate` green.
- **M2 task 0016 (`AndroidProvider` — `launch` + `stop`) done:** `launch` spawns
  `emulator @<avd_name>` (+ `-no-window` / `-wipe-data` / `-no-snapshot-load` / `-gpu <mode>` from
  `LaunchOpts`, + `extra_args` — flags cited from the official emulator command-line reference, not
  invented), streams its output lines onto the `JobHandle`, and polls
  `adb -s <serial> shell getprop sys.boot_completed` until `1`, with a wall-clock 300 s timeout
  (overridable) and a fast-fail if the emulator process exits before boot. The spawned child is
  **held** in an `EmulatorId → child-handle` map on the provider (not dropped after boot) so
  `stop` can reap it — "never orphans the process" — and its stdout stays drained. `stop` does a
  graceful `adb -s <serial> emu kill` (serial found by matching the registry `avd_name` against
  each running emulator's `adb -s <s> emu avd name`), waits, then force-kills the held child as the
  fallback; idempotent when nothing is running. Added a general `FakeChild` "lingering" mode
  (`FakeProcessRunner::on_spawn_lingering`) to `emu-core`'s test fakes so the pure never-boots
  timeout path is testable. 8 new tests; `just validate` green (109 Rust tests total).
- **CI (task 0009) status:** unchanged — still blocked on a GitHub Actions billing/spending-limit
  issue on the account (`sachinshettigar/emumanager`, **Settings → Billing & plans**), not a repo
  problem. Nothing to do here until a human fixes it.
- **M1 tasks 0010–0012** (component catalog, native ports, toolchain bootstrap): see the log
  entries below for full detail — all done and real-network-tested where it matters.
- **M1 task 0013 (Dependencies screen wired to real state) done:** two new `tauri-specta`
  commands (`list_components`, `bootstrap_toolchain` in `src-tauri/src/commands/toolchain.rs`) and
  one typed event (`BootstrapProgress`, wire name `job://bootstrap`) collapsing
  `docs/architecture.md` §4's `job://progress`/`job://log`/`job://done` trio into one channel (no
  real per-job registry exists yet — M3 — and there's only ever one toolchain-bootstrap job at a
  time). The Dependencies screen (`src/routes/Dependencies.tsx`) now shows the real catalog +
  installed state, a working "Install" button, and a live progress/log panel — 5 new Vitest tests.
  This is the **first real construction** of task 0011's native ports (`NativeFs`,
  `NativeDownloader`, `NativeProcessRunner`) — their `#[allow(dead_code)]` blanket is gone.
  **Two real bugs found and fixed** while wiring this for real (neither was catchable by
  `emu-core`'s own unit tests alone): (1) `zip::ZipFile` isn't `Send`, so task 0012's extraction
  loop wasn't `Send`-safe across its own `.await` points — invisible until a real
  `#[tauri::command]` (whose async body *must* be `Send`) called into it; fixed by splitting the
  zip-crate-touching code into a plain synchronous function run via `tokio::task::spawn_blocking`,
  separate from the async `Fs`-writing loop. (2) `specta-typescript` refuses to export `u64` to a
  plain TS `number` (bigint precision-loss guard) — `ComponentInfo.size_bytes` is `u32` instead,
  with a saturating cast at the one construction site.
- **Toolchains:** rustc 1.98.1, pnpm 10.0.0, `just` 1.58 (brew), java 21 (system JDK).
- **Published:** private GitHub repo `sachinshettigar/emumanager` (`main` pushed).
- **Last validated commit:** see `.agent/state.json` `lastValidatedCommit`.
- **Next action:** task `0029` — `src-tauri` logging: `tracing` + a daily-rotating file layer in
  `<data_dir>/logs/`, wired at the top of `setup`; an `export_diagnostics` command that zips the
  recent logs + latest `HostReport` + versions + a redacted `emulators` dump, then `reveal_path`s
  it; an "Export diagnostics" button on the Dependencies screen. Then `0030` (E2E harness).
  Separately: once GitHub billing is fixed, re-watch the next `ci.yml` push run,
  then flip `0009`/`M0` to `done`.

## Milestone checklist

- [~] **M0** Skeleton & gate — tasks 0001–0008 done; 0009 (CI) blocked on a GitHub billing issue,
      not code — see Current state
- [~] M1 Toolchain manager: SDK from zero — tasks 0010–0013 **all done**; milestone itself stays
      in_progress only because one DoD line is deliberately deferred with a documented reason
      (download queue/pause/cancel) — see `MILESTONES.md`
- [~] M2 Create & launch one emulator end-to-end — tasks `0014`–`0017` **all done**; milestone
      stays in_progress only on its one DoD line (a real `tauri-driver` E2E boot), deferred to
      M6's e2e-suite work — see `MILESTONES.md`
- [~] M3 Registry & reliable tracking — **functionally complete**, tasks `0018`–`0021` all `done`:
      schema + typed `Registry` API; `reconcile` + `delete` + kill-safety + the DoD property test;
      shared managed provider + lifecycle commands + exit reap; detail panel + log console. Stays
      `in_progress` on the same M6 `tauri-driver` E2E line as M0/M1/M2.
- [~] M4 Profiles: export / import / recreate — tasks `0022`–`0024` all `done` (engine, IPC,
      Profiles screen). Stays `in_progress` on the export→wipe→import E2E deferred to M6.
- [~] M5 Host readiness & elevated helper — tasks `0025`–`0027` all `done`. Stays `in_progress`
      on the live-CI DoD + Windows verification (M6).
- [~] M6 Cross-platform hardening & packaging — **current milestone**; **unsigned** (ADR 0007).
      `0028` (packaging + `release.yml`) and `0031` (per-SDK install + onboarding + About) **done**;
      `0029`–`0030` todo. The tag-triggered CI run itself is gated on GitHub billing.
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

### 2026-09-08 — session 9 (continued) (Claude Code) — task 0031 done (per-SDK install + first-run onboarding + About) — user request

Three small first-run improvements the user asked for:

- **Each SDK component installs on its own.** `install_component(componentId)` filters the resolved
  catalog to one component and runs `toolchain::bootstrap` for that single-element `wanted` slice
  (a shared `run_bootstrap` helper — `bootstrap_toolchain` was refactored onto it too). The
  Dependencies screen gives every not-installed row its own "Install" button; the top button is now
  "Install all (N)" and appears only when more than one component is missing.
- **First-run onboarding.** A compact `OnboardingChecklist` on the Dashboard: three numbered steps
  (install the SDK → check host readiness → create your first emulator), each ticked when done and
  linking to the screen that resolves it. It unmounts once all three are satisfied. The stale
  "Host detection lands in M5" strip was replaced with a live one-line host verdict.
- **About screen.** `app_info()` (a plain `#[tauri::command]`) returns name / version
  (`CARGO_PKG_VERSION`) / licence (`CARGO_PKG_LICENSE` = `UNLICENSED`, shown honestly as "all
  rights reserved — OSS license TBD") / repo / a 9-line feature summary. New `/about` route + nav
  item + icon.

28 commands. 4 new/changed Vitest specs (38 web tests total), 154 rust tests, `just validate`
green.

### 2026-09-07 — session 9 (continued) (Claude Code) — ADR 0007 (ship unsigned); M6 scoped; task 0028 done (packaging + release.yml)

Project owner: ship v1 installers **unsigned** — no Apple/Windows code-signing certs, users do the
standard first-run OS override. Wrote `docs/adr/0007-ship-unsigned-v1.md`, amended `docs/spec.md`
§6 + §8, and rescoped `MILESTONES.md` M6 (dropped "signed + notarized" / "no Gatekeeper warning"
from the DoD; kept unsigned installers + the signature-verified Tauri updater + the CI matrix).
M6 scoped into `0028` (packaging + `release.yml`), `0029` (logs + diagnostics), `0030` (E2E
harness).

**Task `0028` done.** `src-tauri/tauri.conf.json` now bundles: `bundle.active = true`,
`targets = "all"`, `createUpdaterArtifacts = true`, an `icon` list with real `icon.icns` / `icon.ico`
(regenerated via `pnpm tauri icon`), `deb` deps, NSIS `currentUser`. `plugins.updater` carries a
real generated Ed25519 **public** key (the private key lives only in the session scratchpad — the
release manager regenerates their own before the first tag; `docs/playbooks/release-signing.md`).
`tauri-plugin-updater = "2"` is registered in `run()`; `capabilities/default.json` is
least-privilege (`core:default` + `updater:default`, no wildcard). `.github/workflows/release.yml`
(new) builds **unsigned** installers on a `v*` tag — matrix `macos-latest` ×2 (arm64 + x64) /
`ubuntu-latest` / `windows-latest` via `tauri-apps/tauri-action@v0` — signs the updater artifacts
with the `TAURI_SIGNING_*` secrets, and drafts a pre-release. `actionlint` clean.

`just package` (new recipe = `pnpm tauri build`) ran on this Mac: `EmuManager.app` 16 MB,
`EmuManager_0.1.0_aarch64.dmg` **6.8 MB** (spec §6 wants < 20 MB), plus a signed
`EmuManager.app.tar.gz` + `.sig`. `README.md` got an "Install (pre-release)" section with the
per-OS unsigned first-run steps. `.gitignore` blocks `*.key`. `just validate` green — the workflow
is written + lint-clean only; the tag-triggered CI run still waits on the GitHub Actions billing
block (same one holding `ci.yml`).

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0027 done (host panel + launch-gating); M5 functionally complete

`src/routes/Dependencies.tsx` gets a `HostPanel`: a verdict banner coloured by `verdict`
(`canAccelerate` / `degraded` / `cannotRun`, showing `verdictReason`), four `Tile`s
(virtualization, accelerator `kind · status`, RAM, disk-free) from `probe_host`, and — when there
are fixes — a row per fix with its description, a "Fix it" button for the `scriptable` ones
(`run_helper(fix.id)` → shows the helper `message`; a `cancelled` error → "the prompt was
dismissed"), a "manual" tag for the rest, and a reboot note when `needsReboot`.

Launch-gating: `Dashboard`'s `EmulatorRow` and `EmulatorDetail` both call `useHostReport` (shared
`["host-report"]` query key) and disable "Launch" when `verdict === "cannotRun"`, with
`verdictReason` shown beside the button and as a `title`. `degraded` isn't gated — the banner
already carries that warning. `useRunHelper`'s `onSettled` invalidates the host report, so a fix
re-probes everywhere.

Every screen test that renders a host-report consumer (Dependencies / Dashboard / EmulatorDetail)
gained a `probeHost` (+ `runHelper`) bindings mock — without it `commands.probeHost()` is
`undefined` and the render crashes on `host.error.ipc.message`. 3 new Vitest (Dependencies +2,
Dashboard +1). 154 rust tests, 34 web tests, `just validate` green (`bindings.ts` unchanged — the
commands shipped in `0026`).

**M5 is functionally complete** — `0025` (detection) + `0026` (helper + elevation IPC) + `0027`
(panel + gating). The helper-schema half of the DoD is met (`emu-helper/tests/schema.rs`); the
"live CI runner, no nested virt → `CannotRun`" half is met in *logic* (`build_report`'s unit test)
and the real CI run + the Windows probe/helper verification are deferred to M6. `currentMilestone`
→ M6.

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0026 done (emu-helper + probe_host / run_helper)

`crates/emu-helper/src/actions.rs` — real per-OS subcommand bodies behind `#[cfg(target_os)]`.
macOS: `check` runs `sysctl kern.hv_support`, the rest are `notApplicable`. Linux: `check` opens
`/dev/kvm`, `add-kvm-group` runs `usermod -aG kvm <user>` where `<user>` is `$SUDO_USER` (or
resolved from `$PKEXEC_UID` — never `root`). Windows: best-effort `dism` /
`Get-WindowsOptionalFeature`, **untested on this Mac** — flagged for a Windows CI runner (M6). The
output is one line of camelCase JSON `{command,status,message,needsReboot}`, `status` ∈
`ok`/`failed`/`notApplicable`/`needsReboot`, exit `0` for all but `failed`. `crates/emu-helper/tests/schema.rs`
runs each subcommand as a subprocess and asserts the shape + exit code — the M5 DoD's second half.

`src-tauri/src/commands/host.rs`: `probe_host() -> HostReportDto` (runs `NativeHostProbe::inspect`,
maps the enums to strings and the `u64` RAM/disk to `u32` MB, and writes a best-effort
`host_snapshots` row); `run_helper(fixId) -> HelperOutcomeDto` — rejects non-scriptable ids,
resolves `emu-helper` next to the app binary, and wraps it with an OS elevation prompt via a
**pure, unit-tested** `elevated_argv(helper, sub, os)` (`osascript … with administrator
privileges` / `pkexec` / `powershell Start-Process -Verb RunAs`). A dismissed prompt →
`IpcError` `cancelled`; Windows can't forward the child's stdout so it returns a synthetic
"re-probe" outcome. `emu-host` is now a real `src-tauri` dependency, so its `cargo-machete` ignore
entry is gone (the whole block — every workspace crate is now consumed by `src-tauri`).

26 commands. 154 rust tests, `just validate` green. Frontend hooks are task `0027`.

### 2026-09-06 — session 9 (continued) (Claude Code) — M5 scoped; task 0025 done (emu-host detection)

M5 scoped into `0025` (`emu-host` detection), `0026` (`emu-helper` + `probe_host` / `run_helper`
IPC), `0027` (Dependencies host panel + launch-gating).

**Task `0025` done.** `emu-host` implements `HostProbe`:
- `signals.rs` — `HostSignals` (os / arch / `virtualization` / `accel: AccelSignal` /
  `ram_bytes` / `disk_free_bytes`), the OS-independent input.
- `report.rs` — `build_report(&HostSignals) -> HostReport`, **pure, no `#[cfg]`**. Verdict:
  `CannotRun` for `DisabledInFirmware` / RAM < 4 GB / disk-free < 8 GB / accelerator `Missing`;
  `Degraded` for `NoPermission` / `Disabled` / RAM < 8 GB / disk-free < 25 GB; else
  `CanAccelerate`; `Unknown` signals never hard-block. `fixes[]` cite the real commands
  (`enable-virtualization` non-scriptable+reboot, `add-kvm-group` scriptable, `enable-whpx`
  scriptable+reboot, …). 8 unit tests, including the M5 DoD's "CI runner, no virtualization →
  `CannotRun` + the BIOS fix".
- `probe.rs` — `NativeHostProbe`: RAM/disk via `sysinfo 0.36`; `platform_probe()` per OS —
  macOS `sysctl -n kern.hv_support`, Linux `/dev/kvm` read/write open (`EACCES` → `NoPermission`),
  Windows `powershell Get-WindowsOptionalFeature` (best-effort, **untested on this Mac** — verify
  on a Windows runner in M6). One `#[ignore]` real-machine test; ran it here →
  `os=macos accel=Hvf/Ok verdict=CanAccelerate`.

New `emu-host` deps: `sysinfo` (system + disk only), `async-trait`, `tokio` (dev). 149 rust tests,
`just validate` green.

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0024 done (Profiles screen); M4 functionally complete

`src/routes/Profiles.tsx` rewritten from its static placeholder: a `<label>` drop zone wrapping a
hidden file input (drag-over styling), `FileReader` → `number[]` → `inspect_profile` → a preview
with name / device / image, a **requirement table** (`installed` / `download N MB` per row), the
total-download figure or a "ready" note, and **Apply / Apply & launch** (streams `useEmulatorJob`
just like the Create wizard, links to the Dashboard on success) + **Save to library**. Below it, a
saved-profiles list from `list_profiles` with **Load** (fetches the JSON and re-inspects it) and
**Delete**. A rejected file shows the backend's specific `message` verbatim.

Emulator detail gets an **Export profile** button (`export_profile` → clipboard + a read-only
`<textarea>` — no dialog plugin). The Create wizard's review step gets **Save as profile**, which
assembles the `EmuProfile` JSON client-side from the wizard selection (parsing `imageCoord`) and
calls `save_profile`.

`src/lib/ipc.ts` got the seven profile hooks (`useInspectProfile` / `useApplyProfile` /
`useExportProfile` / `useProfiles` / `useSaveProfile` / `useDeleteProfile` / `getSavedProfile`) —
all now consumed, so `knip` is green. `Profiles.test.tsx` (4 tests). Gotcha: jsdom's `File` has no
`arrayBuffer()`, so `readBytes` uses a `FileReader` promise.

131 rust tests, 31 web tests, `just validate` green. **M4 is functionally complete** — engine
(`0022`), IPC (`0023`), UI (`0024`) all done. It stays `in_progress` only on the DoD E2E
(export → wipe → import → boots again), deferred to M6's e2e-suite line alongside M2's and M3's.
`currentMilestone` → M5.

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0023 done (profile IPC — inspect / apply / export)

`src-tauri/src/commands/profile.rs` (new) wires task `0022`'s engine to the app:

- `inspect_profile(bytes) -> ProfileInspection` — `parse_profile` (serde + `EmuProfile::validate`,
  with **specific** rejection messages: not-JSON, no `schemaVersion`, unsupported version, wrong
  `platform`, bad field) → `emu_core::profile::resolve` against `provider.installed_state()` +
  `image_size_bytes(coord)` (fetches the six `sys-img2-3.xml` manifests). Returns the requirement
  table + total download + `ready`. (`bad-abi.json` — a schema-enum violation — is not caught here;
  the domain `Abi` is wider than the schema, so that stays the frontend's `ajv` job.)
- `apply_profile(bytes, launch) -> String` — `ensure_image` → `provider.create` →
  `provider.set_source(id, Imported { profile_id: <name>, origin_label })` → optional launch,
  streaming on `job://emulator` with `jobId` `apply:<avd>`. Reuses `emulator::{job_handle,
  emit_job, EmulatorJobKind}` (promoted to `pub(crate)`).
- `export_profile(id) -> String` — `EmuProfile::from_emulator(...).to_json_pretty()`.
- Saved profiles: `save_profile` / `list_profiles` / `get_saved_profile` / `delete_profile` over a
  `migrations/0003_profiles.sql` `profiles` table with new `json` + `description` columns. New
  `Registry` methods (`save_profile`/`list_profiles`/`get_profile`/`delete_profile`/`set_source`)
  and `AndroidProvider` accessors (`registry()`, `installed_state()`, `set_source`).

24 commands. Frontend hooks + the Profiles screen are task `0024` (shipping unused hooks trips
`knip`). 3 new tests. `just validate` green (the `bindings.ts` diff is the usual pre-commit regen).

### 2026-09-06 — session 9 (continued) (Claude Code) — M3 closed out; M4 scoped; task 0022 done (profile engine)

M3 tasks `0018`–`0021` flipped to `done`; M3 stays `in_progress` on the same M6 `tauri-driver` E2E
line as M0/M1/M2; `currentMilestone` → M4. M4 (Profiles) scoped into `0022` (engine), `0023`
(IPC + apply), `0024` (screen).

**Task `0022` done (in review)** — the pure profile engine, all in `emu-core`, no IPC:
- `crates/emu-core/src/profile/{mod,resolve}.rs` (new): `resolve(profile, &InstalledState,
  image_installed, image_size_bytes) -> Result<Plan>` — reuses the existing `plan::Plan`
  (`{ diff, create_spec }`, already has `total_download_bytes()`/`is_ready()`) rather than a new
  `RequirementDiff` type. One `Requirement` per component (`CmdlineTools`/`PlatformTools`/`Emulator`
  from `InstalledState`, `SystemImage` from the caller's bool + size). **`platforms;android-NN` is
  not emitted** — task `0019`'s real capture showed `avdmanager create avd` succeeds without it.
  `sanitize_avd_name` moved here (still duplicated in `commands/emulator.rs` — a later cleanup).
- `model/profile.rs`: reverse `From<ImageType>` / `From<Graphics>` impls + `EmuProfile::from_emulator`
  / `from_create_spec` / `to_json_pretty` — the export direction. An adopted emulator's empty
  `device_profile_id` exports as `pixel_6` so the file still schema-validates.
- Schema-sync test `model_round_trip_stays_schema_valid` — a new `jsonschema` dev-dep
  (`default-features = false`, 0.33 for rustc 1.82), round-trips every `valid/*.json` through the
  model and re-validates against `v1.schema.json`. `cargo deny` passes with its ~17 transitive dev
  deps. Runtime schema validation stays the frontend's `ajv`.
- 7 new tests, `just validate` green. `docs/architecture.md` §3 profile-engine note updated.

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0021 done (detail panel + log console); M3 functionally complete

**Per-emulator log.** `AndroidProvider::launch` now tees every streamed line into
`<data_dir>/logs/<avd_name>.log` (truncated at launch start) as well as onto the job — one helper,
`emit_log`, called from `launch`'s framing lines and `wait_for_boot`'s per-line drain (not a second
drain). `Fs` has no append, so `emit_log` does a read-modify-write per line — fine for a boot log
(a few KB); a real `Fs::append` is the follow-up if a continuous post-boot stream is added.
`AndroidProvider::read_log_tail(id, max_lines)` + the `emulator_log_tail` command return the tail
(empty, not an error, when never launched; `NotFound` for an unknown id).

**Detail panel** — `src/routes/EmulatorDetail.tsx` at `/emulator/:id`: `emulator_detail` config +
live state, inline-editable name → `rename_emulator`, a RAM/storage/graphics form → `edit_hardware`
(with the "applied on next AVD recreate" caveat shown), Wipe data / Delete (inline confirm + "also
remove the AVD" checkbox) / Launch-or-Stop, "Open folder" → the `reveal_path` command (chose it
over `@tauri-apps/plugin-opener` — one `std::process` spawn, no plugin dep / capability entry). The
log console shows the `emulator_log_tail` history then appends live `job://emulator` `Log` lines
(deduped against the tail); copy-to-clipboard + "Open AVD folder". gRPC port shows `—` (still not
parsed). Dashboard rows link to the route. `ipc.ts` got the `0020`+`0021` hooks
(`useEmulatorDetail`, `useEmulatorLogTail`, `useRenameEmulator`, `useEditHardware`,
`useDeleteEmulator`, `useWipeEmulatorData`, `revealPath`).

17 commands total. 3 new Rust tests (log tee round-trips through `read_log_tail`, empty for
never-launched, `NotFound`), 4 new Vitest (`EmulatorDetail.test.tsx`). 131 rust tests, 27 web
tests; `just validate` green (the `bindings.ts` diff is the usual pre-commit regen the lefthook
stages).

**M3 is functionally complete** — all six DoD boxes + the property-test DoD (task `0019`) are met.
`.agent/state.json` keeps M3 `in_progress` / tasks `review` until a verification pass flips them,
then M3 → `done` and `currentMilestone` → M4.

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0020 done (shared managed provider + lifecycle commands)

The `AndroidProvider` is no longer rebuilt per command. `src-tauri/src/provider_state.rs` holds a
`ManagedProvider` — a `tokio::sync::OnceCell<Arc<AndroidProvider>>` plus the fixed `(data_dir, os)`
— built in `setup` and `app.manage()`d; the first command that calls `mgr.get().await` builds the
provider (chose `OnceCell` over `block_on` in `setup` so `Registry::open`'s migrations never block
the window; an unsupported host yields a clear `unsupported` error at call time, the app still
launches). Every emulator command now takes `State<'_, ManagedProvider>`.

**The M2 "app exit orphans an emulator" gap is closed.** The shared provider means
`create_emulator`'s spawned-child map persists, so `stop_emulator` actually reaches
`AndroidProvider::stop`'s force-kill fallback, and `run()` switched to
`.build(ctx).run(|handle, event| …)` so a `RunEvent::ExitRequested` handler calls the new
`AndroidProvider::shutdown()` (drains the `running` map, kill+wait each child, per-child 5 s
timeout) inside an 8 s `block_on(timeout(…))`. `reconcile()` also runs once at startup, spawned
from `setup` (non-blocking; best-effort — no logging surface in this crate, the 4 s Dashboard poll
recovers a failure).

New commands (all thin): `reconcile_now` (reconcile + refreshed `EmulatorInfo[]`), `rename_emulator`,
`edit_hardware` (records the row's hardware; a live `config.ini` rewrite / AVD recreate is a
documented follow-up), `delete_emulator(id, wipe)`, `wipe_emulator_data` (refuses while running,
else deletes the AVD's `userdata-qemu.img` / `cache.img` / `snapshots/` etc. so the next launch
rebuilds them — the real `-wipe-data` mechanism), `emulator_detail(id) -> EmulatorDetail`, and
`reveal_path` (`open -R` / `explorer /select,` / `xdg-open`, no new dep). 16 commands total.
New `AndroidProvider` methods: `detail`, `rename`, `set_hardware`, `wipe_data`, `shutdown`.

Frontend: `useReconcileNow` + a Dashboard "Refresh" button + a Vitest (23 web tests). The
detail-panel hooks (`emulator_detail` / rename / edit / delete / wipe / `revealPath`) are held for
task `0021` — shipping them here with no consumer trips `knip`; their backend commands and the
`EmulatorDetail` type are already generated into `bindings.ts`.

128 rust tests, 23 web tests, `just validate` green (the `bindings.ts` diff is the usual
pre-commit regen the lefthook stages).

### 2026-09-06 — session 9 (continued) (Claude Code) — task 0019 done (reconcile + delete + kill-safety)

`AndroidProvider::reconcile()` and `delete()` — the registry now converges to ground truth.

**Real `avdmanager list avd` capture.** No Android SDK on this machine, so captured one the way
`0014`/`0015` captured theirs: downloaded `commandlinetools-mac_arm64-16111833`, `sdkmanager` for
`system-images;android-24;default;x86_64` (399 MiB — smallest modern `default` x86_64), `emulator`,
`platform-tools`; two real `avdmanager create avd` runs + one hand-seeded broken AVD;
`avdmanager list avd` → `tests/fixtures/avdmanager-list-avd.txt`. Quirks encoded in the new
`emu-android::avd_list` parser: `Target:` prints empty even with the platform installed (the
android-version is on the `Based on:` continuation line); blocks separated by nine dashes; broken
AVDs come after a `could not be loaded:` header with only `Name:`/`Path:`/`Error:`, and `-c`
(compact) drops them — so `reconcile` parses the verbose form. Also captured: `avdmanager delete
avd` success/not-found text, and that it removes the whole `.avd` dir (no "keep AVD, wipe userdata"
— that's `0020`'s `wipe_emulator_data`).

**`reconcile()`** parses that output, then: adopts every loadable AVD with no registry row
(`Manual { discovered: true }`, enriched from its `config.ini` — `image.sysdir.1` → `ImageCoord`,
`hw.device.name`, `avd.ini.displayname`, `hw.ramSize`); sets a row whose AVD is missing from the
list, or present-but-un-loadable, to `Error` (never hard-deletes); refreshes every surviving row's
`last_state` / `adb_serial` / `grpc_port` / `pid` from adb. **Kill-safety**: a `ProcessRunner`
can't probe an arbitrary pid, so "not in `adb devices`" is the liveness signal — a `Booting`/
`Running` row whose emulator has vanished from adb (app SIGKILLed mid-boot) is reset to `Stopped`
with `pid` cleared.

**`delete(id, wipe)`**: `wipe = true` → `avdmanager delete avd -n <name>` (removes the whole
`.avd`, userdata included) then `delete_row`; `wipe = false` → `delete_row` only (untrack; a later
`reconcile` re-adopts it as `discovered`). Refuses while running (`Invalid`, "stop it before
deleting"); `NotFound` for an unknown id; a "no Android Virtual Device named" stderr on the wipe
path counts as success (already gone).

**M3 DoD met** — `reconcile_converges_to_ground_truth_over_random_scenarios`: 48 xorshift64
(no `proptest` dep) pseudo-random loadable/broken/running arrangements over a 6-name pool, fresh
provider + registry per iteration; after one `reconcile()` every row's `last_state` equals ground
truth and every loadable AVD is tracked. Fake-driven, in `just validate`.

11 new tests (3 `avd_list`, 8 `provider`), 128 rust tests total, 22 web; `just validate` green.
No IPC surface touched (`bindings.ts` no-op regen — that changes in `0020`). `emu-android` gained
no new deps; the testing fakes needed no changes.

### 2026-09-05 — session 9 (Claude Code) — M3 scoped (0018–0021); task 0018 done (registry schema)

Scoped M3 — Registry & reliable tracking — into four tasks the same granularity as M1/M2:
`0018` full SQLite schema + typed `Registry` API, `0019` `AndroidProvider::reconcile` + `delete` +
kill-safety + the M3 DoD property test, `0020` shared managed provider (`tauri::State`, so the
spawned-child map persists → real force-kill and app-exit reap) + startup reconcile + the
rename/edit-hardware/delete/wipe/`reconcile_now`/`emulator_detail` commands, `0021` the
`/emulator/:id` detail panel + per-emulator log console. Then did `0018`.

**Task `0018` done (in review).** `migrations/0002_registry_m3.sql` `ALTER TABLE ADD COLUMN`s the
M0-minimal `emulators` table up to the full tracked record — `device_profile_id`, `image_coord`,
`hardware_json`, `source_json`, `tags_json`, `notes`, `last_state`, `adb_serial`, `grpc_port`,
`pid`, `launched_at`, `updated_at` — and adds a `host_snapshots` table (M5 fills it; the row type
and schema exist now so it's complete). **Compound values (`Hardware`, `EmulatorSource`, tag list)
are one JSON text column each, not one SQL column per field** — `Hardware` has 7 fields and
`EmulatorSource` is a per-variant-payload enum, so per-column would either lose data or need a
migration on every future struct change; the cost (can't `WHERE` on them in SQL) is one nothing
needs. The `hardware_json` migration default is `Hardware::default()` serialized verbatim; a test
asserts the round trip so a drift in the default is caught.

New `crates/emu-core/src/registry/row.rs`: `EmulatorRow` (fully typed — `Option<ImageCoord>`,
`Hardware`, `EmulatorSource`, `RunState`, `Vec<String>` tags, `OffsetDateTime` ×3, `Option<u16>`
port, `Option<u32>` pid) and `HostSnapshotRow`, both mapped from `SqliteRow` with manual
`try_get` (no `FromRow` derive — the JSON/enum columns need custom decoding anyway). The `Registry`
API is now **typed** end to end: `upsert_emulator(&EmulatorRow)` (`INSERT … ON CONFLICT(id) DO
UPDATE`, keeps `created_at`), `get_row`, `list_rows`, `set_run_fields`, `rename`, `set_hardware`,
`delete_row -> bool`, `insert_host_snapshot`, `latest_host_snapshot`. The old tuple methods
(`insert_emulator` / `get_emulator` / `list_emulators`) are **deleted**; the only caller,
`emu-android::provider`, was updated (`create` builds a full `EmulatorRow`; `launch` / `stop` /
`tracked_states` read the typed rows). `serde_json` added to `emu-core`'s deps, `time` to
`emu-android`'s (one `now_utc()` call).

`.sqlx/` offline cache + `query!` macros — `migrations/0001`'s comment and `MILESTONES.md` M0 both
said "in M3". Still deferred, **pointer moved to M4**: runtime `sqlx::query` + manual row mapping
works and `just validate` is green without a `DATABASE_URL` or a committed cache; adopting the
checked macros now would mean a build-time DB or a checked-in `.sqlx/` for every query in the
crate, for a compile-time-vs-first-test convenience. Updated `MILESTONES.md` M0 line, the
architecture Registry note, and the `Cargo.toml` comment.

117 rust tests (+3 net — registry inline tests 2 → 6: migration-applies + every-column round trip,
upsert preserves `created_at`, `set_run_fields`/`rename`/`set_hardware`/`delete_row`,
`list_rows` ordering + duplicate-`avd_name` rejection, `host_snapshots` round trip), 22 web tests;
`just validate` green. No IPC surface touched — `bindings.ts` regenerated as a no-op diff.

### 2026-09-05 — session 8 (Claude Code) — M2 scoped (0014–0017); all four tasks done, M2 functionally complete

Rebuilt and relaunched the app first (confirmed the M1/task-0013 build runs), then scoped M2 into
four tasks the same granularity as M1's — `.agent/tasks/0014-device-and-image-catalogs.md` through
`0017-create-wizard-and-dashboard.md` — and picked up `0014`.

**Real-source research before writing any parser** (per `AGENTS.md` §6 rule 2 — never invent SDK
behavior): downloaded the real `commandlinetools-mac_arm64` zip for this host and inspected it.
`avdmanager list device`'s own plain-text output (captured for real: 15 real device ids, id/Name/
OEM/Tag columns only) doesn't carry the screen/RAM/sensor data `DeviceProfile` needs — so instead
of scraping that text, the parser reads the XML files `avdmanager` itself loads them from, found by
inspecting `sdklib.core.jar`: `devices.xml` (generic + Small/Medium Phone/Tablet), `nexus.xml`
(Pixel/Nexus — this is where "Pixel 6" actually lives), `wear.xml`, `tv.xml`, `automotive.xml`,
`desktop.xml` (a 7th, `xr.xml`, exists but is out of scope — no XR `FormFactor` variant, not in
`docs/spec.md`). Only `wear.xml`/`tv.xml`/`automotive.xml` carry an explicit `<d:tag-id>`; the
other three don't, so `devices::form_factor_for` infers Phone/Tablet/Foldable from id/name for
those — a documented heuristic, not something the tool itself emits, called out clearly as such in
the module doc so nobody mistakes it for cited SDK behavior later.

System images turned out **not** to live in `repository2-3.xml` at all (confirmed by grep — zero
`system-images` packages in a fresh real fetch) — they ship in one `sys-img2-3.xml` manifest per
Google "tag" family, at URLs curl-verified for real 2026-09-05 (`sysimg::MANIFEST_URLS`). Real
capture surfaced "extension-level" packages (`system-images;android-34-ext12;...`) that our
existing `ImageCoord::from_str` (task `0002`) already can't parse — reused that as the skip signal
instead of writing separate ext-level detection.

Real fixtures, real bytes, trimmed for size (same trim policy as task `0010`'s
`repository2-3.xml`): `crates/emu-android/tests/fixtures/{devices,nexus,wear,tv,automotive,
desktop}.xml` and `sys-img2-3-{android,google-apis-playstore,wear}.xml`. 19 new unit tests, all
fixture-driven, no network, no process spawn (this task is parsers only — locating/extracting these
files from an installed SDK is task `0015`'s job, alongside `ensure_image`/`create` which need the
same install-dir knowledge). `just validate` green; no IPC surface touched, so `bindings.ts` was
regenerated as a no-op diff.

Also found while writing the RAM-unit converter: the three real files use three different units
for `<d:ram unit="...">` — GiB (`nexus.xml`), MiB (`wear.xml`), KiB (`automotive.xml`) — all three
paths are exercised by a real fixture, not just the common one.

`.agent/state.json`: `M2` flipped to `in_progress` (M0/M1/M2 now form one contiguous in-progress
run, still satisfying `progress-check.mjs`'s rule); tasks `0014`–`0017` registered, `0014` → done.

**Task 0015** (`AndroidProvider` — the first real `Provider` impl) followed immediately. Real
research **changed the task's own written plan mid-flight** (`AGENTS.md` §9): `ensure_image` was
scoped to download a system image via `Downloader` and extract it ourselves, mirroring
`cmdline-tools`' bootstrap. Before writing that, downloaded a real, deliberately small (720 MB)
system image (`system-images;android-34;default;x86_64`) and ran `yes | sdkmanager
"system-images;android-34;default;x86_64"` against a real scratch SDK — confirming `sdkmanager`
already fetches, verifies, and unpacks its own packages correctly, unpacking into exactly
`<sdk_root>/system-images/android-34/default/x86_64/` (package path with `;` → `/`, same convention
already visible for `cmdline-tools;latest`), leaving the same `source.properties` marker every real
`sdkmanager` package writes. That's the same "don't reimplement what the tool already does" choice
`bootstrap.rs` made for `platform-tools`/`emulator` — so `ensure_image` became `ProcessRunner`-only,
zero `Downloader`/zip code, and the task file's own scope/acceptance criteria were rewritten to
match before implementing.

`create()` drives a real `avdmanager create avd`. No `--help` exists for this subcommand — the real
flags (`-n`, `-k`, `-d`, `-f`, `--skin`, `-b`, `-g`, `-c`, `-p`) came from a live "unrecognized
flag" usage dump. A full real end-to-end run (against the now-genuinely-installed image) captured:
success is exit-`0`-only with no positive text at all; and three real, distinct `stderr` failure
texts — duplicate name, unknown `-d` device id, and an invalid `-k` package path — each mapped to
its own specific `CoreError` rather than one generic failure. `--force` is deliberately never
passed: a name collision should be a clear error, not a silent overwrite. `EmulatorId::generate()`
was added (a real `ulid` dependency, `cargo deny`-clean) since `Emulator`'s own doc comment already
promised "a ULID string in practice" with nothing generating one yet; `Registry` gained
`insert_emulator`/`get_emulator` against the existing M1-era `emulators` table (no new migration —
the full schema is M3's job).

`AndroidProvider` implements all 8 `Provider` trait methods (Rust requires the complete impl) but
only `ensure_image`/`create` do real work — `list_devices`/`list_images` stub out to task `0017`
(wiring task `0014`'s parsers to a real installed SDK), `launch`/`stop` to task `0016`, `delete`/
`reconcile` to M3. 17 new tests (8 `emu-core`: ULID generation + registry insert/lookup/duplicate-
rejection; 9 `emu-android`: `AndroidProvider`'s no-op/success/failure paths, every real error text
above, and the package-path-to-directory helper), all fixture/fake-driven, no network. Honestly
noted gap: `FakeProcessRunner` has no real filesystem side effects, so no unit test proves
`sdkmanager` actually unpacks files end to end — that's the manual real capture above, not
`cargo test`; a `--ignored` integration test (task `0012`'s own pattern) is the natural next step if
this gap needs closing. `just validate` green throughout; no IPC surface touched, `bindings.ts`
regenerated as a no-op diff both times.

**Task 0016** (`AndroidProvider::launch` + `stop`) closed out the provider's runtime surface.
Emulator flags cited from the official emulator command-line reference
(<https://developer.android.com/studio/run/emulator-commandline>): `emulator @<avd_name>`,
`-no-window`, `-gpu <auto|host|swiftshader_indirect>`, `-no-snapshot-load`, `-wipe-data` — `launch`
passes only what `LaunchOpts` asks for plus `extra_args` (no `-accel`: the emulator's own `auto`
default handles it, host-readiness is M5; no `-no-audio`: a windowed dev emulator may want it).
`launch` spawns via `ProcessRunner::spawn`, drains the child's output onto the `JobHandle` in
~250 ms slices while polling `adb -s <serial> shell getprop sys.boot_completed` until `1`, with a
wall-clock 300 s deadline (`with_boot_timeout` override for tests) and a fast-fail path when the
emulator's output stream reaches EOF (process exited) before boot. The spawned child is **kept** in
an `EmulatorId → Arc<AsyncMutex<Box<dyn ChildProcess>>>` map on the provider rather than dropped:
that's what lets `stop` reap it (acceptance criterion "never orphans the process") and keeps its
stdout drained so a chatty emulator never blocks on a full pipe. `stop` finds the serial by
matching the registry `avd_name` against each running emulator's `adb -s <s> emu avd name` (so it
never kills the wrong one), sends a graceful `adb -s <serial> emu kill`, waits up to 15 s
(`with_stop_timeout`), then force-kills the held child as the documented fallback; it's idempotent
when nothing is running and `NotFound` for an unknown id. `grpc_port` is left `None` (parsing it +
a continuous post-boot log stream are M3's detail-panel / live-console work).

To test the pure "stream stays open, boot never completes, deadline fires" path (impossible with
the old `FakeChild`, which always hit EOF and so took the fast-fail branch first), added a general
`FakeChild` *lingering* mode to `emu-core`'s test fakes: `FakeProcessRunner::on_spawn_lingering` —
after the scripted lines, `next_line()` never resolves until `kill()`, modelling any long-lived
child. 8 new tests (launch: boots / streams-then-boots / exits-before-boot / never-boots-timeout;
stop: `emu kill` to the matched serial / idempotent / unknown-id / force-kill fallback). Honest
gap (in 0016's Notes): no unit test drives a real `emulator`/`adb` — argv, the poll loop, the
timeout branch and the stop fallbacks are all fake-covered; real end-to-end boot is M2's
`tauri-driver` E2E (task 0017) or a later `--ignored` test, and Unix zombie-reap on app-kill is
M3's kill-safety milestone. `just validate` green (109 Rust tests); no IPC surface touched.

**Task 0017** wired the whole thing to the UI and closed out M2. `src-tauri/src/commands/emulator.rs`
adds six `tauri-specta` commands and one `EmulatorJob` event (`job://emulator`), structurally a
copy of `toolchain::BootstrapProgress`. Key shape decisions, all documented in the task file:
`AndroidProvider` is constructed per command (matching how `toolchain.rs` builds its ports per
call) — the trade-off is that `0016`'s spawned-child map doesn't survive between calls, so `stop`
here leans on the graceful `adb emu kill` path; a shared/managed provider is M3. `list_devices` /
`list_images` are implemented in the shell rather than on the `Provider` trait — the trait
signatures can't carry manifest bytes or a `Downloader`, and putting `reqwest` in `emu-android`
would break the "network lives behind a port" rule — so the command reads the installed `sdklib`
jar off disk and feeds it to the new pure `devices::parse_from_jar` (which needed a `zip` dep on
`emu-android`), and fetches the six `sys-img2-3.xml` manifests concurrently with
`futures_util::try_join_all` before `sysimg::parse` — exactly the `toolchain::resolve_catalog`
pattern. `AndroidProvider` gained `sdk_root` (made `pub`), `is_image_installed`, and
`tracked_states()` — a read-only Dashboard view (registry rows + a live `RunState` probed from
`adb`), explicitly **not** `reconcile()` (no adopting/dropping — that's M3); `Registry` gained
`list_emulators()`. `Create.tsx` is now a real 4-step wizard (searchable device list → image list
with installed/size → name+RAM+storage → review → Create / Create & launch, with a live job-log
panel), `Dashboard.tsx` a polled emulator list with per-row Launch/Stop, and `ipc.ts` got the
matching hooks. Scope trims (hardware form is name/RAM/storage only; inline image download folded
into "Create"; "Save as profile" is M4; row uptime needs the M3 launch-timestamp schema;
`ensure_image` progress is a coarse log line, not `sdkmanager`'s own byte bar) are all recorded in
the task file and `MILESTONES.md`. 22 web tests (Create.test.tsx new, Dashboard.test.tsx
rewritten) + 3 new Rust unit tests; `just validate` green (114 Rust tests; the `bindings.ts` diff
is the normal pre-commit regen the lefthook stages). `currentMilestone` advanced to M3.

### 2026-09-05 — session 7 (Claude Code) — M1 task 0013 (Dependencies screen), M1 wrapped up

- **Task 0013 done** — the last M1 task. `src-tauri/src/commands/toolchain.rs` (new), `src/lib/
  ipc.ts`, `src/routes/Dependencies.tsx` rewritten from its M0 static placeholder.
- **Two new commands**: `list_components` (real catalog, task 0010, merged with real installed
  state, task 0012) and `bootstrap_toolchain` (runs the real `bootstrap()` end to end). **One
  typed event**: `BootstrapProgress` (wire name `job://bootstrap`) — a deliberate simplification
  of `docs/architecture.md` §4's three-event scheme into one tagged-enum payload
  (`Progress`/`Log`/`Done`), because there's no real per-job registry anywhere yet (that's M3) and
  only one toolchain-bootstrap job ever runs at a time (a fixed `JOB_ID` constant, not generated).
  Recorded as a deliberate scope decision in the task file, not silently done differently from the
  spec.
- **First real construction of task 0011's native ports.** `src-tauri/src/ports/mod.rs`'s blanket
  `#[allow(dead_code, unused_imports)]` — flagged back in task 0011 as "remove the moment
  something calls these for real" — is gone. Removing it surfaced two more real, pre-existing
  issues it had been silently hiding: a genuinely unused `AsyncWriteExt` import in
  `ports/downloader.rs`'s own tests (fixed), and `SystemClock` truly having no caller yet (given
  its own small, honest, still-documented allow instead — M2's launch tracking is the expected
  first user).
- **Real bug #1: `zip::ZipFile` isn't `Send`.** Task 0012's `extract_cmdline_tools` held a
  `ZipFile` (which wraps a non-`Send` `&mut dyn Read`) across `.await` points. `emu-core`'s own
  `#[tokio::test]`s never required the future to be `Send`, so this shipped unnoticed — a real
  `#[tauri::command]`'s async body *must* be `Send`, and wiring `bootstrap_toolchain` into one is
  what finally caught it, as a compile error pointing deep into `emu-core`. Fixed in
  `crates/emu-core/src/toolchain/bootstrap.rs`: split the zip-crate-touching code into a plain,
  non-`async` function that reads the whole archive into owned data with no `.await` anywhere
  (run via `tokio::task::spawn_blocking`, which also keeps ~140 MB of real decompression off the
  async executor), fully separate from the async loop that writes through `Fs`. Documented in
  both task 0012's and task 0013's Notes, since the bug and the fix both live in 0012's file but
  were only caught while doing 0013's work.
- **Real bug #2: `u64` can't cross the IPC seam.** `specta-typescript` refuses to export
  `u64`/`usize`/etc. to a plain TS `number` (a real bigint-precision-loss guard, not a false
  positive). `ComponentInfo.size_bytes` is `u32` instead of the catalog's `u64`, narrowed with a
  saturating cast at the one construction site — every real M1 archive, and even `docs/spec.md`'s
  largest quoted future system image (~3.5 GB), fits comfortably under `u32::MAX`.
- **`#[tauri::command]` functions can't be `pub use` re-exported** — their hidden
  `__cmd__*`/`__specta__fn__*` sibling items stay at the function's defining module path, which a
  re-export doesn't move. `lib.rs` references `commands::toolchain::{list_components,
  bootstrap_toolchain}` directly instead of through `commands::{...}`.
- **Frontend**: `useComponents`/`useBootstrapToolchain`/`useBootstrapProgress` hooks in
  `src/lib/ipc.ts`; `Dependencies.tsx` shows real installed/not-installed rows (with *where* a
  component was found), a working Install button, and a live log/progress panel. 5 new Vitest
  tests. A global `@tauri-apps/api/event` mock was added to `src/test/setup.ts` (mirroring the
  existing `@tauri-apps/api/core` mock) so route-level tests that render `Dependencies`
  incidentally don't try to reach a real Tauri event host under jsdom.
- **M1 wrap-up**: all four M1 tasks (0010–0013) are done. `MILESTONES.md`'s M1 section still has
  two boxes deliberately left unchecked (download queue/pause/cancel; `sdkmanager --list`
  parsing), each with a written reason and a pointer to when it'll actually matter (M2+) — so
  `.agent/state.json` keeps `M1.status: "in_progress"` rather than overclaiming `"done"`, while
  `currentMilestone` moves to `M2`. Small tooling follow-up: `scripts/progress-check.mjs`'s
  in-progress-milestone rule (relaxed once already in session 6) needed relaxing again slightly —
  "contiguous run, none past `currentMilestone`" instead of "ends at `currentMilestone`" — since
  `currentMilestone` can now be ahead of an M1 that's still legitimately `in_progress`.
- **Not done here**: no real per-job registry (M3); no download pause/resume/cancel in the UI
  (task 0011 never built it into the port either); no `sdkmanager --list` parsing (M2, when real
  system images matter); no M2 task files yet.

### 2026-09-05 — session 6 (Claude Code) — M1 task 0012 (toolchain bootstrap)

- **Task 0012 done** — `crates/emu-core/src/toolchain/{mod,installed_state,bootstrap}.rs`:
  `InstalledState::scan` + `bootstrap()`, the orchestration that turns an empty data dir into a
  working, licensed `sdkmanager`.
- **System-SDK detection built** (the user's mid-session requirement from the previous session):
  `scan()` checks the app-managed `sdk/` dir first, then `ANDROID_SDK_ROOT`, then `ANDROID_HOME`,
  then the OS-conventional Android Studio install path (`~/Library/Android/sdk` macOS,
  `~/Android/Sdk` Linux, `%LOCALAPPDATA%\Android\Sdk` Windows) — each via the same marker-file
  check, env/path lookup as a plain injected closure rather than a new port trait.
  `ComponentLocation` records *where* a component was found (`SdkSource::AppManaged` vs.
  `System(PathBuf)`), not just a bool, so `bootstrap()` (and later `create`/`launch`) know which
  `sdkmanager`/`adb`/`emulator` to actually run — `toolchain::binary_path()` is the public helper
  for that. `bootstrap()` only fetches/installs what's missing from *both* locations; a genuinely
  fully-satisfied `wanted` set is a true no-op (asserted in a unit test — zero process/download
  calls at all, not even the JDK check).
- **Design decision, made while building this**: only `cmdline-tools` is ever downloaded and
  unpacked directly by this module (SHA-1-verified against task 0010's catalog, since
  `Downloader::fetch` only verifies SHA-256 — verification happens at the call site instead of
  changing the trait). `platform-tools` and `emulator` are installed by asking the **real**
  `sdkmanager` to do it (`sdkmanager "platform-tools" "emulator"`, one call, only the still-missing
  ones) — reusing its own resolver/downloader/checksum logic instead of reimplementing it for two
  more components. This is a real simplification the task file's literal wording already implied
  but I confirmed was the right call by actually testing the alternative complexity it avoids.
- **Archive extraction goes through the `Fs` port, not a bypass.** `zip` (pure-Rust, no system
  `unzip`/`tar` dependency — matters for `docs/spec.md` goal 1's "zero external setup") reads the
  archive; every entry is written via `Fs::ensure_dir`/`write_atomic`, so the whole path is
  exercisable with `InMemoryFs` in fake-driven tests. Since `Fs::write_atomic` carries no
  permission mode, the executable bit is missing after extraction — fixed with one real
  `chmod -R +x <bin dir>` call through the existing `ProcessRunner` port (a no-op on Windows,
  where `chmod` isn't even a real binary) rather than inventing a fifth port trait or bypassing
  `Fs` to touch `std::fs` permissions directly.
- **Three real captures, not guesses, this task needed and got:**
  1. Downloaded the real `cmdline-tools` zip and inspected it — its top-level folder is literally
     named `cmdline-tools/`, which must be renamed to `latest/` (an Android placement convention
     the archive itself doesn't encode); confirmed the exec bit really is lost through a plain
     unzip-then-Fs-write round trip.
  2. Ran a real `yes | sdkmanager --licenses` against that real `cmdline-tools` (JDK 21 on this
     machine) end to end: captured the exact `N of N SDK package licenses not accepted.` /
     `N/N: License <id>:` / `Accept? (y/N):` / `All SDK package licenses accepted` shape, and
     confirmed the count (7, that day) is not a stable number to hardcode — `bootstrap()` feeds a
     bounded 50 `y` answers instead of piping `yes` forever.
  3. Ran real `java -version` and confirmed OpenJDK prints its version line to **stderr**, not
     stdout — `ensure_jdk` checks whichever stream is non-empty.
- **JDK decision recorded**: `docs/adr/0006-require-system-jdk.md` — v1 requires a system JDK
  17+ (checked before any download starts, clear actionable error if missing/too old) rather than
  bundling a JRE; `docs/spec.md` §8's open question updated to point at the ADR instead of leaving
  it dangling.
- **The `#[ignore]`d integration test was actually run, not just written.** `emu-core` can't
  depend on `src-tauri`/`tauri` (AGENTS.md §6.1), so `crates/emu-core/tests/toolchain_bootstrap.rs`
  defines small, test-local `TestFs`/`TestDownloader`/`TestProcessRunner` over real
  `tokio::fs`/`reqwest`/`tokio::process` (plus a dev-dependency on `emu-android`'s real catalog
  parser — a supported Cargo dev-dependency cycle, test-build-only). Ran it for real:
  `cargo test -p emu-core --all-features --test toolchain_bootstrap -- --ignored` downloaded the
  real `cmdline-tools` + `platform-tools` from Google into a scratch temp dir and got a working
  `sdkmanager --version` back, ~35s, 2026-09-05.
- **Small tooling fix found and made along the way**: `scripts/progress-check.mjs` only allowed
  one milestone `in_progress` at a time, which broke the moment M1 legitimately started while M0
  stays open purely on the externally-blocked CI task. Relaxed the rule to allow a contiguous run
  of `in_progress` milestones ending at `currentMilestone` (documented inline) rather than fudging
  M0's status to satisfy the checker.
- **68 emu-core unit tests + 1 registry integration test + 1 real `--ignored` integration test,
  all passing.** `just validate` green.
- **Not done here**: no JRE bundling (ADR 0006); no download progress/IPC wiring (task 0013); no
  `sdkmanager --list` output parsing (M2, when system images matter).

### 2026-09-05 — session 5 (Claude Code) — M1 task 0011 (native port impls)

- **Task 0011 done** — real, OS/network-facing implementations of the four leaf ports, living in
  `src-tauri/src/ports/` per the architecture doc's crate-boundary table (these are glue, not
  domain logic, so they don't belong in `emu-core` or `emu-android`).
- `NativeProcessRunner` (`process.rs`): `tokio::process::Command`, no shell. `run()` captures
  stdout/stderr separately via `wait_with_output()`. `spawn()` returns a `ChildProcess` whose
  `next_line()` streams merged stdout+stderr (two reader tasks forwarding into one channel) —
  documented trade-off: once merged, `wait()` can't attribute leftover lines back to their
  original stream, so they fold into `Output::stdout` with `stderr` left empty; every real
  consumer (tailing `emulator`/`sdkmanager` logs) only needs the merged text anyway. Found and
  fixed a real hang-prone bug while writing the tests: stdin must be `Stdio::null()` (not always
  `Stdio::piped()`) when there's nothing to feed, or a child reading stdin to EOF (`cat`,
  `sdkmanager` without `--licenses` input) blocks forever waiting for a write end that never
  closes.
- `NativeDownloader` (`downloader.rs`): `reqwest` streamed to a temp file, SHA-256 hashed
  incrementally, atomic rename on success, progress reported only when the percentage actually
  changes (not per-chunk). Flagged, not resolved: Google's repo manifest (task 0010) only
  publishes SHA-1, this port verifies SHA-256 — task 0012 (the first real caller) has to decide
  how those reconcile.
- `SystemClock` / `NativeFs` (`clock.rs`, `fs.rs`): straightforward `time`/`tokio::fs` wrappers,
  matching the `Fs::write_atomic` temp-file-plus-rename pattern already established.
- **Real bug this task's own `just validate` run found:** `reqwest`'s `rustls` feature (not
  `rustls-tls` — that feature name changed since the task was scoped) pulls in
  `rustls-platform-verifier` → `webpki-root-certs`, licensed `CDLA-Permissive-2.0` — not on
  `deny.toml`'s allow-list. Added it (data-only crate, Mozilla's root CA bundle, not copyleft
  code) with a comment. Verified by actually running `cargo deny check` locally, the same
  discipline as task 0009's CI-bug fixes.
- 10 new tests (all `#[tokio::test]`, no real network — the downloader tests spin up a one-shot
  local `TcpListener` HTTP/1.0 responder). `cargo test -p emumanager --all-features`: 16 passed.
  Full `just validate`: green.
- **Deliberate `#[allow(dead_code, unused_imports)]`** on `src-tauri/src/ports/mod.rs`, scoped
  and commented: no command constructs these yet (that's 0012/0013), and constructing-then-never
  calling them (e.g. a `reqwest::Client` at startup) would add real cost for nothing — reverses
  what the task file originally assumed ("an allow-free construction"), recorded as such in the
  task's own Notes.
- **Not done, by scope:** `NativeDownloader` is one-shot (no pause/resume/cancel/queueing) —
  `MILESTONES.md`'s fuller M1 "Download engine" bullet stays unticked until that's actually
  needed.
- **New requirement from the user, filed into task 0012, not decided/implemented here:**
  `InstalledState` must recognize an Android SDK the machine already has (env vars or the
  OS-conventional Android Studio path) and skip downloading a component that's already satisfied
  there — not just check the app's own managed dir every time.

### 2026-09-05 — session 4 (Claude Code) — M1 task 0010 (SDK component catalog)

- **Task 0010 done** — the first real M1 behavior: turning Google's Android SDK repository
  manifest into typed, host-matched components.
- `crates/emu-core/src/model/component.rs` (new): `ComponentId` (`CmdlineTools`, `PlatformTools`,
  `Emulator` — the 3 components M1 needs, with `.repo_path()` returning the exact `sdkmanager`
  package path and `.m1_set()` in install order), `HostOs`/`HostArch` (tag strings matching the
  manifest's `<host-os>`/`<host-arch>` exactly, plus `::current()` from `std::env::consts`),
  `Component { id, version, url, size_bytes, sha1 }`.
- `crates/emu-android/src/catalog.rs` (new): `parse(xml, os, arch) -> Result<Vec<Component>>` —
  a `roxmltree` DOM walk (chosen over `quick-xml` — a tree fits "find by attribute, read a few
  children" better than a streaming/serde model here) that finds each `<remotePackage>`, joins
  its `<revision>` into a version string, and picks the `<archive>` whose `<host-os>`/optional
  `<host-arch>` matches. Missing package or no matching archive → a real `CoreError`, not a
  panic. Relative `<url>` values are resolved against `catalog::BASE_URL`.
- **Real fixture, not hand-written:** `crates/emu-android/tests/fixtures/repository2-3.xml` is
  Google's actual manifest (`curl`'d from <https://dl.google.com/android/repository/repository2-3.xml>,
  captured 2026-09-05), trimmed to the 3 `<remotePackage>` elements this parser reads — every
  byte inside them is verbatim. Real-data quirks this surfaced (documented in task 0010's Notes):
  `<url>` is a bare filename, not absolute; `<host-arch>` is *absent*, not `"any"`, when one
  archive covers every arch; `<revision>` doesn't always have `<micro>`; checksums are SHA-1
  only; `emulator` has no `windows`/`aarch64` archive at all (used as the real "no match" test
  case instead of inventing one).
- 10 new tests (linux/x64, macOS/arm64 incl. arch-specific vs. arch-agnostic archives,
  windows/x64, missing-package, missing-archive-for-host, malformed-XML), all fixture-driven, no
  network. `cargo test -p emu-core -p emu-android --all-features`, `scripts/emu-core-no-tauri.sh`,
  `just check-fast`, and full `just validate` all green.
- Carried forward, not decided here (flagged in tasks `0011`/`0012`): the manifest's SHA-1 vs.
  `Downloader::fetch`'s SHA-256 verification; whether v1 bundles a JRE (per `docs/spec.md` §5.1)
  or requires a system JDK (modern `cmdline-tools` needs one either way).
- Also this session: diagnosed the `0009` CI run that never started as a GitHub Actions
  billing/spending-limit block (see Current state) — not a code fix, documented and parked.
  Discarded an unrelated stray `dist/index.html` diff left over from an earlier local
  `tauri build` in this working tree before committing.
- Scoped and filed the rest of M1 as tasks `0011` (native port impls), `0012` (toolchain
  bootstrap + `InstalledState`), `0013` (Dependencies screen wired to real state).

### 2026-09-05 — session 3 (Claude Code) — M0 task 0009 (CI workflows)

- **Task 0009 in review** — the `.github/workflows/*.yml` + composite action scaffolded in
  session 0 are now actually runnable, plus one addition (`dependabot.yml`).
- **Fixed the load-bearing gap:** `.github/actions/setup/action.yml` had no Linux system
  packages for Tauri v2 (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libssl-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`, `build-essential`) — every rust
  build/test/clippy step on `ubuntu-latest`, not just `tauri build`, would have failed without
  them. Added, cited <https://v2.tauri.app/start/prerequisites/#linux>.
- Tool installs: `just`/`cargo-nextest`/`cargo-deny`/`cargo-machete`/`cargo-llvm-cov` via
  `taiki-e/install-action@v2` (prebuilt, fast); `sqlx-cli`/`typos-cli` via
  `cargo install --locked || true` (best-effort, never breaks the job); `actionlint` via its own
  `download-actionlint.bash` release script (no crates.io package) — verified clean locally on
  all 4 workflow files with the same script.
- `nightly-integration.yml`: `just e2e` now skips cleanly when no `playwright.config.*` exists
  (no e2e suite yet — deferred to M2+) instead of failing every night.
- `ci.yml`: added `timeout-minutes` to both jobs.
- `docs/playbooks/milestone-review.md`: concrete branch-protection required-check names, flagged
  as a human (or explicitly-directed agent) action — not applied here.
- **Deferred, documented:** `gitleaks`/`lychee` CI installers (no reliable non-cargo prebuilt
  path found quickly; `just validate` already skips them cleanly); auto-filing a GitHub issue on
  nightly failure (a `::warning::` annotation stands in).
- **Not yet ticked `done`** — waiting to observe the first live push-triggered `ci.yml` run on
  `main` before closing the task and M0.

### 2026-09-05 — session 3 (Claude Code) — M0 task 0008 (lefthook git hooks)

- **Task 0008 done** — `lefthook` is an npm dev-dep (`2.1.12`, bundles the binary); `just setup`
  runs `pnpm exec lefthook install`, `just hooks` re-installs. Hooks are **active on this repo
  now**.
- `pre-commit`: `cargo fmt` on staged `*.rs` + `prettier --write` on staged web files (re-added);
  `just bindings` when `*.{rs,toml}` staged (re-stages `src/lib/bindings.ts`); `just db-prepare`
  guarded on `.sqlx/` + `cargo-sqlx` (no-op until M3); `gitleaks protect` guarded on the binary
  (skip line otherwise); `just progress`.
- `commit-msg`: Conventional Commits regex (verified: valid → exit 0, `bad` → exit 1).
- `pre-push`: `just validate`; auto-skips when `CI` set; `LEFTHOOK=0 git push` override.
- `docs/testing-and-validation.md` gained a "Git hooks" table.

### 2026-09-05 — session 3 (Claude Code) — M0 task 0007 (`just validate` gate)

- **Task 0007 done** — `just validate` runs the full static-analysis + test matrix and is
  **green** on macOS. Run-all/report-all (not fail-fast): every step runs, failures are tallied,
  non-zero exit at the end. `docs/testing-and-validation.md` documents the matrix.
- Core checks (always run, must pass): progress-check, emuprofile schema via `ajv` (full mode —
  valid + invalid fixtures), `emu-core-no-tauri`, `cargo fmt`, `cargo clippy --all-targets
  --all-features -D warnings`, rust tests `--all-features`, `just bindings` + `git diff`,
  `pnpm typecheck/lint/format:check/test`, `knip`, `markdownlint-cli2`.
- Optional tools skip with `(skip: <tool> …)` when absent (never fail the gate): `cargo-nextest`
  / `-deny` / `-machete` / `-llvm-cov`, `sqlx prepare --check` (M3), `typos`, `actionlint`,
  `gitleaks`, `lychee`. `just setup` installs the cargo ones + `typos-cli` and tries `brew` for
  the rest; CI (0009) installs them in the runner.
- New dev-deps: `knip` 5.39.2 (`knip.json`), `markdownlint-cli2` 0.15.0, `ajv` 8.20.0. Dropped
  unused `@testing-library/user-event`.
- knip-driven cleanup: `src/lib/ipc.ts` now exports only `usePing` + `IpcCallError`; added the
  missing `ajv` dep.
- `.markdownlint-cli2.yaml` relaxed (MD022/028/031/032/036/040 off — they only hit pre-existing
  docs). New: `.gitleaks.toml`, `lychee.toml`, `.config/nextest.toml`.
- `lastValidatedCommit` set (this commit).

### 2026-09-05 — session 3 (Claude Code) — M0 task 0005 (sqlx registry bootstrap)

- **Task 0005 done** — `emu-core::registry::Registry::open(data_dir)` creates
  `<data_dir>/db.sqlite` (WAL + foreign keys), runs `sqlx::migrate!("../../migrations")`,
  re-open is idempotent. `migrations/0001_init.sql`: `emulators`, `images`, `profiles`, `jobs`
  (minimal — full schema is M3).
- `sqlx` 0.8, `default-features = false`, features `runtime-tokio` + `sqlite` (bundled, no
  system lib) + `migrate` + `macros` (only for `migrate!`). **No `query!` macros** → no
  `DATABASE_URL`, no `.sqlx/` cache; `validate.sh`'s `sqlx prepare --check` step is now gated on
  `[[ -d .sqlx ]]`. Runtime `query_as` used in the test.
- `CoreError::Db { detail }` added (code `db_error`). 1 integration test
  (`tests/registry_open.rs`); `SQLX_OFFLINE=true cargo build -p emu-core` clean.
- `just db-migrate` simplified to forward-only `cargo sqlx migrate add`.
- **Also this session:** added `@tauri-apps/cli` 2.11.4 so `just dev` runs — verified the
  window launches (Vite :1420 + the Rust shell). Commit `b496976`.
- Deviation: `.sqlx/` + compile-time-checked queries deferred to M3; `Registry` not yet opened
  from `src-tauri` startup (no consumer until M3); `lefthook.yml` `db-prepare` hook needs the
  same `[[ -d .sqlx ]]` guard — flagged for task 0008.

### 2026-09-05 — session 3 (Claude Code) — M0 task 0004 (tauri-specta IPC seam)

- **Task 0004 done** — the typed Rust↔TS seam is live.
- `src-tauri`: `commands::ping(name) -> Result<Pong, IpcError>`; `Pong { message, version }`;
  `IpcError { code, message, details }` with `impl From<emu_core::CoreError>` so `code` is the
  pinned `CoreError::code()` string. `specta_builder()` is the single source of truth — `run()`
  mounts it, the `export::export_bindings` test renders it to `src/lib/bindings.ts`.
- `just bindings` = that test (`cargo test -p emumanager --lib export::export_bindings`); output
  is deterministic, so `git diff --exit-code src/lib/bindings.ts` is the CI check.
- Frontend: `src/lib/ipc.ts` unwraps the tauri-specta `{status}` envelope into a value or an
  `IpcCallError` (a real `Error` carrying the backend `IpcError` on `.ipc`); `usePing()` is a
  TanStack Query hook; `main.tsx` gains a `QueryClientProvider`; Dashboard shows
  "pong, EmuManager from v0.1.0".
- Tests: 6 rust (`ipc_error`, `commands`) + 4 web (`ipc.test.tsx`, `Dashboard.test.tsx`), total
  11 web. `cargo test --workspace --all-features`, `clippy -D warnings`, `fmt`, `pnpm lint`,
  `pnpm format:check`, `scripts/emu-core-no-tauri.sh` all green.
- Versions pinned together: `tauri-specta =2.0.0-rc.25`, `specta =2.0.0-rc.25`,
  `specta-typescript =0.0.12`, `@tauri-apps/api 2.11.1`, `@tanstack/react-query 5.102.8`.
- Deviations: `IpcError.details` exports as TS `unknown` (specta refuses `serde_json::Number`);
  `#[derive(specta::Type)]` on `emu-core` DTOs still deferred — no `emu-core` type crosses IPC
  until M1, and 0004's scope is `src-tauri` + frontend. The generated file is excluded from
  eslint/prettier/coverage.

### 2026-09-05 — session 2 (Claude Code) — M0 task 0002 (emu-core ports & models)

- **Task 0002 done** — the `emu-core` domain layer, no real behaviour.
- `model/`: `DeviceProfile`, `SystemImage` + `ImageCoord`/`ImageType`/`Abi` (Display/FromStr
  round-tripping the `sdkmanager` package path), `Emulator`/`Hardware`/`EmulatorSource`/
  `LiveState`, `HostReport` + verdict/fixes, `Job`/`JobHandle`/`Progress`, `Plan`/`Requirement`/
  `CreateSpec`, `EmuProfile` (mirrors `schemas/emuprofile/v1.schema.json`, with `From`
  conversions to the domain types).
- `ports.rs`: `ProcessRunner` (+ `ChildProcess`), `Downloader`, `HostProbe`, `Clock`, `Fs` —
  all `async-trait`, object-safe. `provider.rs`: the `Provider` trait + `LaunchOpts` /
  `RunningHandle`.
- `error.rs`: `CoreError` grew to 9 variants, each with a pinned `code()` string.
- `testing/` (feature `testing`): `FakeProcessRunner`, `FakeDownloader` (real SHA-256),
  `FakeClock`, `InMemoryFs` — all behaviour-tested.
- **39 tests** green with and without `--all-features`; `clippy -D warnings`, `fmt`,
  `emu-core-no-tauri.sh` all green.
- Deviation: `specta::Type` derives deferred to task 0004 (needs the `tauri-specta`/`specta`
  version pin). serde-only for now.

### 2026-09-05 — session 2 (Claude Code) — M0 task 0003 (frontend app shell)

- **Task 0003 done** — Vite 5 + React 18 + TypeScript (strict, project references) app shell.
- Router (`createBrowserRouter`) with `/`, `/create`, `/dependencies`, `/profiles`; `Sidebar`
  (`NavLink`) highlights the active route via `aria-current`; nav list single-sourced in
  `src/nav.ts`. Static placeholder content per screen pointing at the milestone that fills it in.
- `src/styles/tokens.css`: `--em-*` palette (blue `#2f6db3`, green `#2f8a5f`, amber `#b07d2b`,
  neutrals) + light / `[data-theme]` / `prefers-color-scheme` / `prefers-reduced-motion`.
  Tailwind 3.4 surfaces them as semantic utilities (`bg-surface`, `text-muted`, …) — no raw hex
  in components.
- ESLint 9 flat config (typescript-eslint strict + stylistic type-checked, react-hooks,
  react-refresh); Prettier; Vitest 2 + Testing Library (7 tests: 4 route renders + 3 sidebar).
- Verified: `pnpm typecheck`, `pnpm lint`, `pnpm format:check`, `pnpm test`, `pnpm build`, and
  `just check-fast` (now runs the web half) all green. `cargo build --workspace` still green.
- Deviation: `eslint-plugin-import` deferred (flat-config/resolver friction, no M0 value).
- `tauri.conf.json` gained `devUrl` + `beforeDevCommand` / `beforeBuildCommand`.

### 2026-09-05 — session 2 (Claude Code) — M0 task 0006 (task runner)

- **Task 0006 done** — `justfile` finished: every `AGENTS.md` §4 recipe present with a
  `just --list` description; `bindings` recipe de-stubbed off the wrong `-p app` crate name to a
  graceful no-op until task 0004; Windows/WSL guidance in the header.
- Added `package.json` (scripts-only mirror: `typecheck`/`lint`/`format:check`/`test`/`build`/
  `validate`; deps + real Vite config land in task 0003).
- `scripts/validate.sh` + `scripts/check-fast.sh` now guard web steps on `node_modules` so the
  bare `package.json` doesn't break the gate before `pnpm install`.
- `Makefile` target list widened to the full recipe set.
- Verified: `just --summary`, `just progress`, `just check-fast`, `just bindings`, `just test-web`
  all exit 0.

### 2026-09-04 — session 1 (Claude Code) — repo published + M0 task 0001

- Published private repo `sachinshettigar/emumanager`; `git init` + `main` pushed.
- Installed toolchains on the dev machine (rustc 1.98.1, pnpm 11.25.0, just 1.58.0).
- **Task 0001 done** — Cargo workspace (`src-tauri` + `emu-core`/`emu-android`/`emu-host`/
  `emu-helper`). `cargo build --workspace`, `clippy --all-targets -D warnings`, `fmt --check`,
  `cargo test -p emu-core` all green. `emu-helper` CLI stubs `check` / `enable-whpx` /
  `enable-aehd` / `add-kvm-group` emit `not_implemented` JSON.
- Deviations recorded in the task file: identifier `com.emumanager.desktop` (Tauri rejects
  `.app`); placeholder icons via `scripts/gen-placeholder-icons.mjs`; placeholder `dist/index.html`.

### 2026-09-04 — session 0 (Claude Code) — scaffold

- Set product scope to **B: Android only** (ADR 0003) and stack to **Tauri v2 + Rust + React/TS**
  (ADR 0002).
- Created the harness-agnostic project structure (ADR 0005): `AGENTS.md` + per-tool stubs,
  `docs/` (spec, architecture, ADRs 0001–0005, glossary, domain model), `MILESTONES.md`,
  `.agent/` working state, `docs/playbooks/` + `docs/prompts/`, tooling config
  (`justfile`, `lefthook.yml`, linters), `schemas/emuprofile/v1.schema.json` + fixtures, CI
  workflows.
- Design wireframes moved to `docs/design/`.
- No code yet. M0 code tasks (`0001`–`0009`) are written and `todo`.
- Next agent: start at task `0001`; run `just progress` to sanity-check state.
