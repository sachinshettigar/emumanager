# EmuManager — architecture

Companion to `docs/spec.md`. Describes the components, their boundaries, and the main data flows.
Decisions with trade-offs are recorded in `docs/adr/`.

## 1. High-level shape

```
┌─────────────────────────────────────────────────────────────────┐
│ Tauri v2 desktop app                                             │
│                                                                 │
│  Frontend  (src/)  —  Vite + React + TS                          │
│   Screens: Dashboard · Create · Dependencies · Profiles          │
│   TanStack Query wraps the typed IPC bindings; Zustand for       │
│   local UI state; event stream -> job progress + logs            │
│                    │  invoke() + events, all typed by            │
│                    │  tauri-specta (src/lib/bindings.ts)         │
│  Shell  (src-tauri/)  —  commands, event emitters, DI wiring     │
│                    │                                             │
│  Core  (crates/emu-core)  —  no tauri dep                        │
│   • Orchestrator: jobs (download/create/boot) as async tasks     │
│     emitting progress events                                     │
│   • Registry: SQLite (sqlx) + reconcile-with-reality             │
│   • Profile engine: .emuprofile parse/validate/resolve/apply     │
│   • Toolchain manager: catalog, download, checksum, license      │
│   • Ports (traits): Provider, ProcessRunner, Downloader,         │
│     HostProbe, Clock, Fs  — fakes for tests                      │
│                    │                                             │
│  ┌──────────────┬──────────────────┬───────────────────────┐     │
│  │ emu-android  │ emu-host         │ emu-helper (separate   │     │
│  │ Provider impl│ HostProbe impl   │ elevated binary)      │     │
│  │ sdkmanager   │ KVM / WHPX / HVF │ enable-whpx,          │     │
│  │ avdmanager   │ cpuid, disk, ram │ add-kvm-group, ...    │     │
│  │ emulator,adb │                  │                       │     │
│  └──────────────┴──────────────────┴───────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
        │                         │
   dl.google.com (SDK repo,       OS (elevation prompt, once)
   system images, checksums)

Data dir  (directories crate: e.g. %LOCALAPPDATA%\EmuManager, ~/Library/Application Support/EmuManager, ~/.local/share/emumanager)
  sdk/                cmdline-tools, platform-tools, emulator, jre, system-images
  avd/               ANDROID_AVD_HOME
  db.sqlite          registry
  downloads/         in-flight + verified archives
  profiles/          saved .emuprofile files
  logs/              rotating app + per-emulator logs
```

## 2. Crate boundaries

| Crate | Responsibility | May depend on |
| --- | --- | --- |
| `emu-core` | All domain logic and orchestration. Defines **ports** (traits) for every side effect. Pure, deterministic, fully unit-testable. | serde, sqlx, thiserror, tracing, tokio |
| `emu-android` | Implements `Provider` by driving `sdkmanager`/`avdmanager`/`emulator`/`adb` via the injected `ProcessRunner`. Output parsing lives here, tested against captured fixtures. | emu-core |
| `emu-host` | Implements `HostProbe`: virtualization + accelerator detection per OS, disk/RAM, "can we accelerate?" verdict. | emu-core |
| `emu-helper` | Standalone tiny binary. Subcommands: `enable-whpx`, `enable-aehd`, `add-kvm-group`, `check`. Invoked with OS elevation on explicit user action, returns structured JSON. No shared state with the app beyond argv/stdout. | (minimal — clap, serde) |
| `src-tauri` | Tauri commands + events; constructs the real port impls and injects them into `emu-core`; maps domain errors to typed IPC errors. **No business logic.** | all of the above, tauri, tauri-specta |

**The rule that keeps this honest:** `emu-core` has `tauri` in neither `[dependencies]` nor
`[dev-dependencies]`. CI greps for it.

## 3. Key abstractions (in `emu-core`)

```rust
// Ports — every side effect is one of these, so tests inject fakes.
trait ProcessRunner  { async fn run(&self, cmd: Command) -> Result<Output>; async fn spawn(&self, cmd: Command) -> Result<Child>; }
trait Downloader     { async fn fetch(&self, url: &Url, into: &Path, progress: ProgressSink) -> Result<Verified>; }
trait HostProbe      { async fn inspect(&self) -> Result<HostReport>; }
trait Clock          { fn now(&self) -> OffsetDateTime; }
trait Fs             { /* dir ensure, atomic write, list */ }

// The provider surface the UI ultimately drives (only Android impl for scope B).
trait Provider {
    async fn list_devices(&self) -> Result<Vec<DeviceProfile>>;
    async fn list_images(&self, filter: ImageFilter) -> Result<Vec<SystemImage>>;
    async fn ensure_image(&self, coord: ImageCoord, job: JobHandle) -> Result<()>;
    async fn create(&self, spec: CreateSpec) -> Result<EmulatorId>;
    async fn launch(&self, id: EmulatorId, opts: LaunchOpts, job: JobHandle) -> Result<RunningHandle>;
    async fn stop(&self, id: EmulatorId) -> Result<()>;
    async fn delete(&self, id: EmulatorId, wipe: bool) -> Result<()>;
    async fn reconcile(&self) -> Result<Vec<LiveState>>;
}
```

- **Jobs / Orchestrator:** long operations return a `JobId` immediately; progress + log lines are
  pushed to a broadcast channel that `src-tauri` forwards as a Tauri event
  (`job://progress`, `job://log`, `job://done`). The UI subscribes per job.
- **Registry:** `sqlx` with SQLite. Tables: `emulators`, `images`, `profiles`, `jobs`,
  `host_snapshots` (schema: `migrations/0001_init.sql` + `0002_registry_m3.sql`). It stores *our*
  metadata (source profile, tags, notes, hardware, timestamps, last-known run state); compound
  values are JSON text columns so a struct change needs no migration. `reconcile()` re-reads ground
  truth from `avdmanager`/`adb` and updates `last_state` so the DB never drifts. The typed
  `Registry` API (`EmulatorRow` in/out) lives in `crates/emu-core/src/registry/`. Queries are
  runtime-checked (`sqlx::query` + `SqliteRow::try_get`); a committed `.sqlx/` offline cache +
  `query!` macros are deferred to M4 (task `0018` Notes). `reconcile()` (task `0019`,
  `emu-android::AndroidProvider`) parses `avdmanager list avd` (`emu-android::avd_list`), **adopts**
  loadable AVDs it doesn't yet track (`Manual { discovered: true }`), **flags** rows whose AVD is
  gone or un-loadable as `Error` (never hard-deletes — that's `Provider::delete`), and applies
  **kill-safety**: a `Booting`/`Running` row absent from `adb devices` is reset to `Stopped`.
- **Toolchain manager:** a static descriptor of required components + their Google repo coords;
  resolves the repository XML for versions; downloads via `Downloader` with SHA verification;
  writes license-hash files to accept licenses; exposes `InstalledState` for the Dependencies
  screen.
- **Profile engine** (`crates/emu-core/src/profile/`, task `0022`): `EmuProfile` serde struct
  (`model::profile`) ↔ `schemas/emuprofile/v1.schema.json`, kept in sync by
  `model_round_trip_stays_schema_valid` (a `jsonschema` dev-dep test — round-trips every valid
  fixture through the model and re-validates). `profile::resolve(profile, &InstalledState,
  image_installed, image_size_bytes) -> Plan { diff: Vec<Requirement>, create_spec: CreateSpec }` —
  pure, effect-free (the caller supplies the two SDK facts). The reverse (`EmuProfile::from_emulator`
  / `from_create_spec`) is the export direction. `apply` (task `0023`) runs the plan through the
  provider. `platforms;android-NN` is deliberately not a requirement — `avdmanager create avd`
  doesn't need it.

## 4. IPC contract

- `tauri-specta` generates `src/lib/bindings.ts` from `#[tauri::command]` signatures and the
  `#[derive(specta::Type)]` models. This is the **only** Rust↔TS interface. Hand-editing it is a
  CI failure (`git diff --exit-code` after `just bindings`).
- Commands are thin: validate input → call `emu-core` → map `CoreError` to a serializable
  `IpcError { code, message, details }`. `code` is a stable enum the UI switches on.
- Events: `job://progress { jobId, phase, pct, etaSecs }`, `job://log { jobId, stream, line }`,
  `job://done { jobId, result }`, `registry://changed`, `host://changed`.

## 5. Main flows

**Create + launch**
1. UI calls `create_emulator(spec)`. Shell → `Orchestrator::create_and_launch`.
2. Core: `Provider::ensure_image` (Toolchain manager downloads if missing, emits progress).
3. Core: `Provider::create` → `avdmanager create avd ...` via `ProcessRunner`; parse id.
4. Registry insert; emit `registry://changed`.
5. `Provider::launch` → spawn `emulator @name` with accel + graphics flags; stream stdout to
   `job://log`; poll `adb` until boot completes; store serial/ports.
6. `job://done { running }`. UI moves the row to "Running".

**Import profile**
1. UI calls `inspect_profile(bytes)` → parse + JSON-Schema validate → `resolve()` against
   `InstalledState` → return `RequirementDiff` (present / needs-download + sizes).
2. User confirms → `apply_profile(plan)` → downloads → `create` → tracked instance.

**Host readiness / fix**
1. `probe_host()` → `HostReport { virtualization, accelerator, disk, ram, verdict, fixes[] }`.
2. A `fix` with `scriptable: true` → UI calls `run_helper(fix)` → `src-tauri` invokes `emu-helper`
   with OS elevation → structured result → re-probe.

## 6. Testing architecture

| Level | What | Tooling | In `just validate`? |
| --- | --- | --- | --- |
| Rust unit | `emu-core` logic with fake ports; `emu-android` parsers vs. fixtures; `emu-host` verdict logic | `cargo nextest`, `insta` snapshots | yes |
| Schema | `.emuprofile` schema self-valid + every fixture in `schemas/emuprofile/fixtures/{valid,invalid}` | `jsonschema` (Rust) + `ajv` (node) | yes |
| Contract | `src/lib/bindings.ts` matches Rust (`just bindings` clean); `.sqlx/` current | `git diff --exit-code` | yes |
| Frontend unit | components + query hooks with a mocked bindings module | Vitest + Testing Library | yes |
| Integration | real `sdkmanager`/`avdmanager` against a scratch SDK dir | `cargo test -- --ignored`, `just test-integration` | no (nightly CI) |
| E2E | launch app, create + boot an emulator, assert dashboard state | Playwright (UI), `tauri-driver` + WebdriverIO (full, Linux/Windows only) | no (per-milestone + release) |

Fakes live in `emu-core/src/testing/` behind a `testing` feature. No test in `just validate`
touches the network or spawns a real Android binary.

## 7. Deferred / future

- Remote-Mac or cloud-emulator provider behind the same `Provider` trait.
- Same-arch snapshot export/import.
- Team profile registry (share via URL, not just file).
