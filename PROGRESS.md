# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **Milestone:** M1 — Toolchain manager: SDK from zero (M0 stays open only on the CI task, see
  below; work moved on to M1 per user direction)
- **Phase:** M0's app skeleton, `just validate` gate, and git hooks are solid and green locally.
  M0 task `0009` (CI) is **blocked, not broken** — see below. M1 started: task `0010` (SDK
  component catalog) **done**; `0011`–`0013` scoped and queued.
- **CI (task 0009) status:** the workflows themselves are fine — a live run did catch and this
  session fixed three real bugs (SPDX license field, wildcard path deps, unmaintained
  advisories; commit `a97606e`, verified locally with the real tools). The *next* push-triggered
  run (33884961977) never started at all: GitHub Actions reported "recent account payments have
  failed or your spending limit needs to be increased" — a billing/spending-limit block on the
  `sachinshettigar/emumanager` account, not a repo problem. Needs a human to fix it in GitHub
  **Settings → Billing & plans**; nothing left to do here until then. Deprioritized per user
  direction ("skip ci cd for now, build locally").
- **Local build confirmed working** as an alternative to CI: `pnpm tauri build --debug` succeeds
  and `target/debug/emumanager` launches (verified as a running process).
- **M1 task 0010 (SDK component catalog) done:** `emu-core::model::component` adds
  `Component`/`ComponentId`/`HostOs`/`HostArch`; `emu_android::catalog::parse()` reads Google's
  real repository manifest and resolves `cmdline-tools;latest`/`platform-tools`/`emulator` for a
  given host, picking the right archive by `<host-os>`/`<host-arch>`. Real fixture captured from
  <https://dl.google.com/android/repository/repository2-3.xml> (trimmed, bytes verbatim), 10 new
  tests, no network in tests.
- **M1 task 0011 (native port impls) done:** `src-tauri/src/ports/`: `NativeProcessRunner`
  (`tokio::process`, merged stdout/stderr line streaming, real `kill()`), `NativeDownloader`
  (`reqwest` 0.13 + `rustls`, streamed to a temp file, SHA-256 verified, atomic rename,
  bounded-rate progress), `SystemClock`, `NativeFs` (`tokio::fs`, atomic write). 10 new tests,
  including a one-shot local TCP HTTP responder for the downloader (no real network call). Not
  wired into any command yet (that's 0012/0013) — a scoped, documented `#[allow(dead_code)]`
  stands in rather than constructing-and-discarding real OS resources for nothing.
- **Toolchains:** installed on this machine — rustc 1.98.1, pnpm 10.0.0, `just` 1.58 (via brew;
  was missing at the start of this session).
- **Published:** private GitHub repo `sachinshettigar/emumanager` (`main` pushed).
- **Last validated commit:** see `.agent/state.json` `lastValidatedCommit`.
- **Next action:** task `0012` (toolchain bootstrap + `InstalledState` in `emu-core`) — carries
  forward two decisions flagged but not made yet (SHA-1-vs-SHA-256 verification;
  bundled-JRE-vs-system-JDK), plus a new explicit requirement from the user: `InstalledState`
  must recognize an already-installed system Android SDK (`ANDROID_HOME`/`ANDROID_SDK_ROOT` or
  the OS-conventional Android Studio path) and skip downloading anything already satisfied there.
  Separately: once GitHub billing is fixed, re-watch the next `ci.yml` push run, then flip
  `0009`/`M0` to `done`.

## Milestone checklist

- [~] **M0** Skeleton & gate — tasks 0001–0008 done; 0009 (CI) blocked on a GitHub billing issue,
      not code — see Current state
- [~] M1 Toolchain manager: SDK from zero — tasks 0010, 0011 done; 0012, 0013 queued
- [ ] M2 Create & launch one emulator end-to-end
- [ ] M3 Registry & reliable tracking
- [ ] M4 Profiles: export / import / recreate
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

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
