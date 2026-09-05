# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **Milestone:** M2 — Create & launch one emulator end-to-end (M0 and M1 both stay open on
  deliberately-deferred/blocked items, see below; `currentMilestone` moved on per user direction)
- **Phase:** M1 is functionally complete — tasks `0010`–`0013` all **done**. M1 itself stays
  `in_progress` in `.agent/state.json` because one of its own written DoD lines is intentionally
  deferred (download queue/pause/cancel — see the Milestone checklist below), not because anything
  is broken. M2 is scoped into 4 tasks (`0014`–`0017`, `.agent/tasks/`); `0014`, `0015` and `0016`
  are **done** — only `0017` (IPC + Create wizard + Dashboard) is left.
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
- **Next action:** task `0017` — `tauri-specta` commands
  (`list_devices`/`list_images`/`create_emulator`/`launch_emulator`/`stop_emulator`) + a
  generalized job event, wiring the Create wizard (`src/routes/Create.tsx`) and Dashboard
  (`src/routes/Dashboard.tsx`) to the real `AndroidProvider`. This also wires
  `AndroidProvider::list_devices`/`list_images` for real (locate the installed `sdklib.core.jar`,
  extract the `devices*.xml` entries, fetch `sysimg::MANIFEST_URLS`). Separately: once GitHub
  billing is fixed, re-watch the next `ci.yml` push run, then flip `0009`/`M0` to `done`.

## Milestone checklist

- [~] **M0** Skeleton & gate — tasks 0001–0008 done; 0009 (CI) blocked on a GitHub billing issue,
      not code — see Current state
- [~] M1 Toolchain manager: SDK from zero — tasks 0010–0013 **all done**; milestone itself stays
      in_progress only because one DoD line is deliberately deferred with a documented reason
      (download queue/pause/cancel) — see `MILESTONES.md`
- [~] M2 Create & launch one emulator end-to-end — tasks `0014` (device + system-image catalogs),
      `0015` (`AndroidProvider` ensure_image + create) and `0016` (`AndroidProvider` launch + stop)
      done; `0017` (IPC + Create wizard + Dashboard) not started
- [ ] M3 Registry & reliable tracking
- [ ] M4 Profiles: export / import / recreate
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

### 2026-09-05 — session 8 (Claude Code) — M2 scoped (0014–0017); tasks 0014, 0015 and 0016 done

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
