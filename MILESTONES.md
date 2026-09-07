# Milestones

Production vision → shippable slices. Each milestone has a **Definition of Done (DoD)**: every box
must be checked *and* `just validate` green before the milestone is "done" in `.agent/state.json`.
Coverage gate ratchets up as noted.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done

---

## M0 — Skeleton & gate  (coverage gate: n/a)

Goal: an empty Tauri app that builds on all three OSes and a validation gate that runs.

- [x] Cargo workspace: `src-tauri` + `crates/emu-core|emu-android|emu-host|emu-helper` compile
- [x] Vite + React + TS (strict) frontend renders an app shell with the 4 nav routes (static)
- [x] `tauri-specta` wired: one `ping` command, `src/lib/bindings.ts` generated and used
- [x] `sqlx` set up with an initial migration; `Registry::open` migrates a SQLite DB in the data
      dir (`.sqlx/` offline cache + `query!` macros deferred — now to M4, see task `0018` Notes:
      runtime queries work and `just validate` is green without it)
- [x] `justfile` with every recipe in `AGENTS.md` §4; `package.json` script mirrors
- [x] `lefthook` installed (npm dev-dep); pre-commit runs fmt + `bindings` + `progress` (guarded
      `db-prepare`/`gitleaks`), commit-msg lints Conventional Commits, pre-push runs `just validate`
- [x] `just validate` runs and passes: fmt, `tsc`, `clippy -D warnings`, tests, vitest, schema
      fixtures (ajv), markdownlint, `knip` all run green; `cargo nextest/deny/machete`, `typos`,
      `actionlint`, `gitleaks`, `lychee` skip with a message until `just setup` / CI installs them
- [~] CI `ci.yml` (fast + 3-OS gate) and `schema.yml` written, `actionlint`-clean; awaiting the
      first live push-triggered run on `main` to confirm green before ticking this. Real bugs
      the one run that did start found (SPDX license, wildcard path deps, unmaintained
      advisories) are fixed and verified locally (commit `a97606e`); the next push-triggered run
      (33884961977) never started — GitHub Actions billing/spending-limit block on the account,
      not a workflow problem. Needs a human to fix billing in GitHub Settings → Billing & plans;
      deprioritized for now in favor of local builds + M1.
- [x] `emu-core` has no `tauri` dependency (`scripts/emu-core-no-tauri.sh`; CI job pending task 0009)
- [x] `scripts/progress-check.mjs` passes (`just progress`)

DoD: fresh clone → `just setup && just validate` passes on all three OSes in CI; `just dev` opens
a window with the 4 (empty) screens.

---

## M1 — Toolchain manager: SDK from zero  (coverage gate: 60%)

Goal: from a machine with nothing, the app installs everything needed to make emulators.

- [x] Component catalog: resolve versions from Google's repository XML (behind `Downloader`)
- [ ] Download engine: queued, resumable, `pause`/`cancel`, SHA-256 verify, progress events —
      **partial**: one-shot fetch+verify+progress lands in task 0011 and is live in the
      Dependencies screen (task 0013); queueing/pause/resume/cancel are still not built — not
      needed by anything through M1, revisit when real multi-GB system-image downloads (M2+) make
      it a real gap
- [x] Bootstrap: fetch `cmdline-tools` into the data dir; run `sdkmanager` through it — **no
      bundled JRE**, a system JDK 17+ is required instead (`docs/adr/0006-require-system-jdk.md`)
- [x] Install `platform-tools`, `emulator`; accept licenses non-interactively
- [x] `InstalledState` scan of the data dir (+ any existing system SDK) drives the Dependencies
      screen
- [x] Dependencies screen wired to real data: components list + live install progress. **Partial**:
      no storage-breakdown view yet (not part of task 0013's scope; revisit if a real need shows up)
- [x] `emu-android` parsers for the SDK component + system-image catalogs, tested against captured
      fixtures — repo XML component catalog (task `0010`) and system-image manifests (task `0014`,
      M2). **Superseded, not literally met**: system images turned out not to be in
      `sdkmanager --list`'s underlying `repository2-3.xml` at all — task `0014` parses Google's
      real per-tag `sys-img2-3.xml` manifests directly instead (see its Notes); the plain-text
      `sdkmanager --list` output itself is still unparsed and not needed
- [x] Unit tests with fake `Downloader`/`ProcessRunner`; no network in `just validate`

DoD: on a clean CI job with an empty data dir, a `--ignored` integration test downloads
`cmdline-tools` + `platform-tools` and `sdkmanager --version` succeeds through the app's managed
install. **Met** (task 0012) — `cargo test -p emu-core --all-features --test toolchain_bootstrap
-- --ignored` does exactly this against a real scratch temp dir; run for real 2026-09-05.

---

## M2 — Create & launch one emulator end-to-end  (coverage gate: 62%)

- [x] `list_devices` + `list_images` commands + parsers/fixtures — parsers (task `0014`,
      `emu-android::devices`/`sysimg` + `devices::parse_from_jar`), IPC commands (task `0017`,
      `commands::emulator`) fetching the real `sdklib` jar + `sys-img2-3.xml` manifests
- [x] `ensure_image` — delegates to a real `sdkmanager` install (task `0015`); progress is a
      single "downloading (can take minutes)" log line, not a byte bar — **partial**: streaming
      `sdkmanager`'s own progress is a nicer-UX follow-up (spawn + parse), noted in task `0015`
- [x] `create` → `avdmanager create avd -n/-k/-d`; parse result; minimal registry row (task `0015`)
- [x] `launch` → spawn `emulator @name` (+ `-no-window`/`-gpu`/… from `LaunchOpts`); stream log
      lines onto the job (task `0016`)
- [x] Poll `adb` for boot-complete; capture serial + pid into `RunningHandle` (task `0016`).
      `grpc_port` + a continuous post-boot log stream are M3's detail-panel work
- [x] Create wizard screen wired: device picker, image picker (installed / download size shown),
      hardware, review (task `0017`). Inline download-from-the-image-picker folded into "Create"
      (it runs `ensure_image` first) rather than a separate button
- [x] Dashboard lists tracked emulators; row shows Stopped/Booting/Running, polled every 4 s
      (task `0017`). Uptime is not shown yet (needs the launch timestamp — M3 registry schema)
- [x] `stop` works (`adb emu kill` + force-kill fallback, task `0016`). "App exit doesn't orphan"
      — **resolved in M3 task `0020`**: the provider is now shared `tauri::State`, so its
      child-handle map persists and a `RunEvent::ExitRequested` handler reaps every child on quit.

DoD: E2E (Linux + Windows, `tauri-driver`): launch app → create a Play Store x86_64 emulator →
assert it reaches Running in the dashboard → stop it. **Not met** — no e2e suite yet (deferred to
M6 "E2E suite runs in CI", same as the rest of the harness). The flow is wired end to end and its
pieces are covered by fake-driven Rust tests + Vitest; a real booted-emulator run is the manual /
`--ignored` gap noted in tasks `0015`/`0016`. M2 stays `in_progress` on this one line.

---

## M3 — Registry & reliable tracking  (coverage gate: 65%)

- [x] Full SQLite schema (emulators, images, profiles, jobs, host_snapshots) + migrations —
      `migrations/0002_registry_m3.sql` grows `emulators` to the full tracked record and adds
      `host_snapshots`; behind a typed `Registry` API (`EmulatorRow` in/out) in
      `crates/emu-core/src/registry/`. Task `0018`. Compound values (`Hardware`, `EmulatorSource`,
      tags) are JSON columns, not one column per field — see the task Notes.
- [x] `reconcile()` **on demand and on startup** — `AndroidProvider::reconcile` (task `0019`):
      parses `avdmanager list avd`, adopts loadable AVDs with no row (`Manual { discovered: true }`,
      enriched from `config.ini`), flags rows whose AVD is missing or un-loadable as `Error`
      (never hard-deletes), refreshes `last_state` / serial / port / pid from adb. Run once at
      startup (spawned from `setup`) and via the `reconcile_now` command / Dashboard "Refresh"
      button (task `0020`).
- [x] Detail panel — `/emulator/:id` (`src/routes/EmulatorDetail.tsx`, task `0021`): image coord +
      API + Play Store, RAM/storage/graphics (editable → `edit_hardware`), adb serial, source,
      created/updated, "Open folder" (`reveal_path`), inline rename, Wipe/Delete/Launch-or-Stop.
      gRPC port shows `—` (not parsed yet). Snapshot management is M7.
- [x] Wipe data, delete (with/without AVD removal), rename, edit hardware — commands all in place:
      `delete_emulator(id, wipe)` (task `0019`), `wipe_emulator_data` (deletes the AVD's writable
      images so the next launch rebuilds them), `rename_emulator`, `edit_hardware` (records the row;
      a live `config.ini` rewrite / AVD recreate is a documented follow-up) — task `0020`. Panel
      wiring is task `0021`.
- [x] Per-emulator log console (task `0021`): `launch` tees its output stream to
      `<data_dir>/logs/<avd>.log`, `emulator_log_tail` reads the history tail, live lines come from
      the `job://emulator` `Log` event; copy-to-clipboard + "Open AVD folder". (A continuous
      post-boot stream — beyond the boot window — would need an `Fs::append` port method; noted.)
- [x] Kill-safety: a `Booting`/`Running` registry row whose emulator is no longer in `adb devices`
      (app SIGKILLed mid-boot) is reset to `Stopped` with `pid` cleared by `reconcile()` (task
      `0019`) — tested (`reconcile_kill_safety_resets_a_stale_running_row`).

DoD: property/integration test that randomly creates/launches/kills and asserts the registry
always converges to ground truth after `reconcile()`. **Met** (task `0019`) —
`reconcile_converges_to_ground_truth_over_random_scenarios`: 48 pseudo-random loadable/broken/
running arrangements, each asserts every row's `last_state` == ground truth and every loadable AVD
is tracked. Fake-driven, in `just validate`.

**All six boxes + the DoD are met** — every M3 task (`0018`–`0021`) is `done`. The milestone itself
stays `in_progress` in `.agent/state.json`, consistent with M0/M1/M2: the one thing left is a real
`tauri-driver` E2E boot, which is deferred to M6's "E2E suite runs in CI" line along with the rest
of the harness. `currentMilestone` has moved to M4.

---

## M4 — Profiles: export / import / recreate  (coverage gate: 68%)

- [x] `EmuProfile` structs ↔ `schemas/emuprofile/v1.schema.json`, kept in sync by a test —
      `model_round_trip_stays_schema_valid` (`jsonschema` dev-dep), task `0022`.
- [x] Export from an emulator and from the Create wizard — `export_profile` command + a "Export
      profile" button on the detail panel (task `0024`); "Save as profile" in the wizard's review
      step (builds the `EmuProfile` client-side); `EmuProfile::from_emulator` / `from_create_spec`
      (task `0022`).
- [x] `inspect_profile(bytes)` → parse + validate → `resolve()` → diff — `emu_core::profile::resolve`
      (task `0022`) behind the `inspect_profile` command (task `0023`), rendered as the drop-zone
      preview (task `0024`).
- [x] `apply_profile` → ensure image → `create` → tracked instance with `source: Imported`
      (task `0023`), wired to the Profiles screen's Apply / Apply & launch (task `0024`).
- [x] Profiles screen: drop zone + file input, import preview with the requirement table + total
      download, saved-profiles list with Load/Delete, "Save to library" (task `0024`).
- [x] Reject non-android / unknown-schema profiles with a specific message — `parse_profile`
      (task `0023`): not-JSON / no `schemaVersion` / unsupported version / wrong platform / bad
      field, each its own message, surfaced on the Profiles screen.
- [~] Round-trip test — the pure half is done
      (`export_from_an_emulator_round_trips_the_recipe_fields`, task `0022`). The full
      export → wipe → import → boots-again flow is the DoD E2E, deferred to M6's e2e-suite line
      (same as M2/M3).

DoD: E2E — export a profile, delete the emulator and its image, import the profile, emulator boots
again. **Deferred to M6** ("E2E suite runs in CI") along with M2's and M3's E2E lines — everything
else in M4 is done and covered by Rust unit + Vitest tests.

---

## M5 — Host readiness & elevated helper  (coverage gate: 70%)

- [x] `emu-host` detection per OS → `HostReport` + `verdict` + `fixes[]` — `crates/emu-host/src/`
      (task `0025`): a pure `build_report(&HostSignals)` (8 unit tests incl. the DoD's CI-runner
      case) + `NativeHostProbe` gathering signals per OS (`sysinfo` RAM/disk; `sysctl` /
      `/dev/kvm` / `powershell` for virtualization + accelerator). **Windows probe untested** — M6 CI.
- [x] `emu-helper` binary: `check` / `enable-whpx` / `enable-aehd` / `add-kvm-group`, real per-OS
      `#[cfg]` bodies, camelCase `{command,status,message,needsReboot}` JSON, `notApplicable` off
      platform (task `0026`). Output-schema test (`tests/schema.rs`) — the DoD's second half.
- [x] `run_helper(fix)` invokes `emu-helper` with OS elevation via a pure `elevated_argv` wrapper
      (`osascript … with administrator privileges` / `pkexec` / `Start-Process -Verb RunAs`), then
      re-probes; a dismissed prompt → `cancelled` (task `0026`). **Elevation path untested end to
      end** (manual / `--ignored`); Windows can't capture the child's stdout (synthetic outcome).
- [x] Dependencies-screen host panel: verdict banner, four tiles, per-fix "Fix it" buttons +
      manual-steps text + reboot notes (task `0027`).
- [x] Graceful degradation: only *launch* is gated (`cannotRun` disables it with the reason on the
      Dashboard + detail panel); the component list / create / emulator list all still work with
      no accelerator (task `0027`).

DoD: on a CI runner without nested virt, the host panel correctly reports `CannotRun` with the
right reason and the right fix; helper `check` output schema is tested. **Helper schema test: met**
(`emu-helper/tests/schema.rs`). **CI-runner verdict: logic met** (`build_report`'s
`ci_runner_without_virtualization_cannot_run_and_offers_the_bios_fix`); the live-CI assertion is
deferred to M6 along with the rest of the harness. M5 stays `in_progress` on that one line.

---

## M6 — Cross-platform hardening & packaging  (coverage gate: 72%)

Scope narrowed by **ADR 0007**: v1 ships **unsigned** installers (no Apple/Windows code-signing
certs). OS signing/notarization is a later config + secrets change. The Tauri updater keeps its own
Ed25519 signature.

- [ ] CI matrix builds **unsigned** installers: `.dmg`, `.msi`/NSIS, `.AppImage` + `.deb`
      (`bundle.active = true`, `bundle.targets` per OS)
- [ ] Tauri updater configured (Ed25519 key, public key in `tauri.conf.json`); `release.yml`
      publishes `latest.json` + per-artifact `.sig` on a tag
- [ ] Least-privilege Tauri v2 capabilities audited; no wildcard fs/shell scopes
- [ ] E2E suite (`tauri-driver` + WebdriverIO/Playwright) exists and runs in CI on Linux + Windows
      each PR — this is where M2/M3/M4/M5's deferred "real boot" DoD lines land
- [ ] Rotating local logs (`tracing` + a file layer); `export_diagnostics` command → a redacted
      zip (logs, `HostReport`, versions — no tokens/paths-with-usernames)
- [ ] README + download docs: the unsigned first-run steps per OS (right-click → Open / SmartScreen)
- [ ] Arch coverage: Apple Silicon + Intel mac, x86_64 Linux/Windows all exercised in the matrix

DoD: a tagged pre-release produces **unsigned** installers for all three OSes from CI, and the app
auto-updates from the previous pre-release (updater signature verified). Still gated on GitHub
Actions billing (see M0) — the code, config and workflows land now, marked so the moment CI is
unblocked it's one flip.

---

## M7 — Feature-complete v1.0  (coverage gate: 75%)

- [ ] Full device catalog incl. Wear / TV / Automotive; custom hardware profile editor
      *(partially done early in the M6 8-item batch: the device step is grouped by form factor
      with per-group counts — task 0037. The reusable custom **hardware/device profile editor**
      is still open.)*
- [ ] Live device inspector — logcat + storage/battery/network
      *(done early — task 0038: an `adb logcat -v threadtime` stream with level / tag / text
      filters + pause/clear, and a facts strip (model, Android version, battery, `/data` usage)
      on the emulator detail panel. Network detail beyond that is still open.)*
- [ ] Snapshot management (create/boot-from/delete) within a single machine
- [ ] Headless launch mode + Wayland window handling resolved (spec open question)
- [ ] APK install into a running emulator (drag-drop)
- [ ] Settings: data-dir location, proxy, telemetry opt-in (default off), theme
- [ ] Accessibility pass (keyboard, reduced-motion, color-scheme); i18n scaffold
- [ ] Docs: user guide, troubleshooting, `.emuprofile` reference
- [ ] All `docs/spec.md` §7 success criteria demonstrably met

DoD: the three success criteria in `docs/spec.md` §7 pass in CI / on real machines; changelog and
release notes written; `LICENSE` chosen.
