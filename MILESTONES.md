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
      dir (`.sqlx/` offline cache deferred to M3 — no `query!` macros yet)
- [x] `justfile` with every recipe in `AGENTS.md` §4; `package.json` script mirrors
- [x] `lefthook` installed (npm dev-dep); pre-commit runs fmt + `bindings` + `progress` (guarded
      `db-prepare`/`gitleaks`), commit-msg lints Conventional Commits, pre-push runs `just validate`
- [x] `just validate` runs and passes: fmt, `tsc`, `clippy -D warnings`, tests, vitest, schema
      fixtures (ajv), markdownlint, `knip` all run green; `cargo nextest/deny/machete`, `typos`,
      `actionlint`, `gitleaks`, `lychee` skip with a message until `just setup` / CI installs them
- [~] CI `ci.yml` (fast + 3-OS gate) and `schema.yml` written, `actionlint`-clean; awaiting the
      first live push-triggered run on `main` to confirm green before ticking this
- [x] `emu-core` has no `tauri` dependency (`scripts/emu-core-no-tauri.sh`; CI job pending task 0009)
- [x] `scripts/progress-check.mjs` passes (`just progress`)

DoD: fresh clone → `just setup && just validate` passes on all three OSes in CI; `just dev` opens
a window with the 4 (empty) screens.

---

## M1 — Toolchain manager: SDK from zero  (coverage gate: 60%)

Goal: from a machine with nothing, the app installs everything needed to make emulators.

- [ ] Component catalog: resolve versions from Google's repository XML (behind `Downloader`)
- [ ] Download engine: queued, resumable, `pause`/`cancel`, SHA-256 verify, progress events
- [ ] Bootstrap: fetch `cmdline-tools` + JRE into the data dir; run `sdkmanager` through it
- [ ] Install `platform-tools`, `emulator`; accept licenses non-interactively
- [ ] `InstalledState` scan of the data dir (versions, sizes) drives the Dependencies screen
- [ ] Dependencies screen wired to real data: components list, active downloads, storage breakdown
- [ ] `emu-android` parsers for `sdkmanager --list` / repo XML, tested against captured fixtures
- [ ] Unit tests with fake `Downloader`/`ProcessRunner`; no network in `just validate`

DoD: on a clean CI job with an empty data dir, a `--ignored` integration test downloads
`cmdline-tools` + `platform-tools` and `sdkmanager --version` succeeds through the app's managed
install.

---

## M2 — Create & launch one emulator end-to-end  (coverage gate: 62%)

- [ ] `list_devices` (`avdmanager list device`) + `list_images` (filtered) commands + parsers/fixtures
- [ ] `ensure_image` downloads a chosen `system-images;...` with progress
- [ ] `create` → `avdmanager create avd` with hardware flags; parse result; register
- [ ] `launch` → spawn `emulator @name` with accelerator + graphics flags; stream logs to `job://log`
- [ ] Poll `adb` for boot-complete; capture serial + ports; emit `job://done`
- [ ] Create wizard screen wired: device picker, image picker (with inline download), hardware, review
- [ ] Dashboard shows the new emulator; row flips Stopped→Booting→Running with uptime
- [ ] `stop` works; app exit doesn't orphan the emulator process

DoD: E2E (Linux + Windows, `tauri-driver`): launch app → create a Play Store x86_64 emulator →
assert it reaches Running in the dashboard → stop it. Zero terminal commands in the flow.

---

## M3 — Registry & reliable tracking  (coverage gate: 65%)

- [ ] Full SQLite schema (emulators, images, profiles, jobs, host_snapshots) + migrations
- [ ] `reconcile()` on startup and on demand: adopt out-of-band AVDs, drop vanished ones, refresh state
- [ ] Detail panel: image coord, RAM/storage, adb serial, gRPC port, snapshot, source; open folder
- [ ] Wipe data, delete (with/without AVD removal), rename, edit hardware (recreate if needed)
- [ ] Per-emulator log console with history tail + live stream; copy / open log file
- [ ] Kill-safety: SIGKILL the app mid-boot → next start reconciles to a correct state (tested)

DoD: property/integration test that randomly creates/launches/kills and asserts the registry
always converges to ground truth after `reconcile()`.

---

## M4 — Profiles: export / import / recreate  (coverage gate: 68%)

- [ ] `EmuProfile` structs ↔ `schemas/emuprofile/v1.schema.json`, kept in sync by a test
- [ ] Export from an emulator and from the create wizard ("Save as profile")
- [ ] `inspect_profile(bytes)` → parse + JSON-Schema validate → `resolve()` → `RequirementDiff`
- [ ] `apply_profile(plan)` → downloads → `create` → tracked instance with `source: Imported`
- [ ] Profiles screen: drop zone, import preview with requirement diff + sizes, saved list, actions
- [ ] Reject non-android / unknown-schema profiles with a specific message
- [ ] Round-trip test: export → wipe → import → equivalent emulator (device/image/hardware match)

DoD: E2E — export a profile, delete the emulator and its image, import the profile, emulator boots
again. Bonus CI job: export on Linux, import on Windows.

---

## M5 — Host readiness & elevated helper  (coverage gate: 70%)

- [ ] `emu-host` detection per OS: virtualization (cpuid/registry/sysctl), accelerator kind+status,
      disk, RAM → `HostReport` + `verdict` + `fixes[]`
- [ ] `emu-helper` binary: `check`, `enable-whpx`, `enable-aehd`, `add-kvm-group`; structured JSON out
- [ ] `run_helper(fix)` invokes it with OS elevation (UAC / `pkexec`/`sudo`), then re-probes
- [ ] Dependencies screen host panel: live tiles, per-fix buttons, reboot/firmware checklist
- [ ] Graceful degradation: no accelerator → app still lists/creates, warns about speed, blocks
      launch with a clear reason + link to the fix

DoD: on a CI runner without nested virt, the host panel correctly reports `CannotRun` with the
right reason and the right fix; helper `check` output schema is tested.

---

## M6 — Cross-platform hardening & packaging  (coverage gate: 72%)

- [ ] CI matrix builds installers: `.dmg` (signed+notarized), `.msi`/NSIS (signed), `.AppImage` + `.deb`
- [ ] Tauri updater configured; `release.yml` publishes update artifacts + signatures
- [ ] Least-privilege Tauri v2 capabilities audited; no wildcard fs/shell scopes
- [ ] E2E suite runs in CI on Linux + Windows each PR; macOS smoke via Playwright + a manual checklist
- [ ] Crash/日志: rotating logs, "export diagnostics" (redacted) command
- [ ] Arch coverage: Apple Silicon + Intel mac, x86_64 Linux/Windows all exercised

DoD: a tagged pre-release produces installers for all three OSes from CI; installing the macOS
build shows no Gatekeeper warning; the app auto-updates from the previous pre-release.

---

## M7 — Feature-complete v1.0  (coverage gate: 75%)

- [ ] Full device catalog incl. Wear / TV / Automotive; custom hardware profile editor
- [ ] Snapshot management (create/boot-from/delete) within a single machine
- [ ] Headless launch mode + Wayland window handling resolved (spec open question)
- [ ] APK install into a running emulator (drag-drop)
- [ ] Settings: data-dir location, proxy, telemetry opt-in (default off), theme
- [ ] Accessibility pass (keyboard, reduced-motion, color-scheme); i18n scaffold
- [ ] Docs: user guide, troubleshooting, `.emuprofile` reference
- [ ] All `docs/spec.md` §7 success criteria demonstrably met

DoD: the three success criteria in `docs/spec.md` §7 pass in CI / on real machines; changelog and
release notes written; `LICENSE` chosen.
