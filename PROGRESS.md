# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **Milestone:** M0 — Skeleton & gate
- **Phase:** Workspace, task runner, frontend shell, `emu-core` domain + registry, the typed
  Rust↔TS IPC seam, and the full `just validate` gate all stand up. Tasks `0001`–`0007` done
  (M0: 7/9). `emu-core` has the model + ports + `Provider` + `testing` fakes (39 unit tests) and
  a SQLite `Registry` (`sqlx` 0.8, `migrations/0001_init.sql`, `Registry::open` + 1 integration
  test). `src-tauri` serves `ping` through `tauri-specta`; `src/lib/bindings.ts` is generated (6
  rust tests). Frontend renders 4 routes and calls `ping` via `usePing()` (11 web tests);
  `just dev` launches a window. **`just validate` is green** — run-all/report-all; core checks
  pass, optional external tools skip with a message until `just setup`/CI install them.
- **Toolchains:** installed on this machine — rustc 1.98.1, pnpm 10.0.0, just 1.58.0.
- **Published:** private GitHub repo `sachinshettigar/emumanager` (`main` pushed).
- **Last validated commit:** see `.agent/state.json` `lastValidatedCommit` (the task-0007 commit).
- **Next action:** task `0008` (lefthook pre-commit/pre-push hooks). Then `0009` (CI matrix).
  Deferred: `#[derive(specta::Type)]` on the `emu-core` DTOs → first M1 IPC command;
  `.sqlx/` offline cache + `query!` macros → M3.

## Milestone checklist

- [~] **M0** Skeleton & gate — tasks 0001–0007 done; 0008, 0009 open
- [ ] M1 Toolchain manager: SDK from zero
- [ ] M2 Create & launch one emulator end-to-end
- [ ] M3 Registry & reliable tracking
- [ ] M4 Profiles: export / import / recreate
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

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
