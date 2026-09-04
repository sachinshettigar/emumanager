# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **Milestone:** M0 — Skeleton & gate
- **Phase:** Workspace, task runner, frontend shell, and the `emu-core` domain layer all stand
  up. Tasks `0001`, `0002`, `0003`, `0006` done. `emu-core` has the full model + port traits +
  `Provider` + `testing` fakes (39 tests). Frontend shell renders 4 routes (7 tests).
  `cargo test --workspace --all-features`, `clippy -D warnings`, `fmt`, and the `pnpm` chain all
  green.
- **Toolchains:** installed on this machine — rustc 1.98.1, pnpm 10.0.0, just 1.58.0.
- **Published:** private GitHub repo `sachinshettigar/emumanager` (`main` pushed).
- **Last validated commit:** _pending — `just validate` not fully wired until task 0007; the
  individual gates (`cargo test/clippy/fmt`, `pnpm` chain, `just check-fast`) are green._
- **Next action:** task `0004` (tauri-specta: `ping` command + generated `src/lib/bindings.ts`;
  also adds `#[derive(specta::Type)]` across the `emu-core` DTOs). Then `0005 → 0007 → 0008 →
  0009`.

## Milestone checklist

- [~] **M0** Skeleton & gate — tasks 0001–0003, 0006 done; 0004, 0005, 0007–0009 open
- [ ] M1 Toolchain manager: SDK from zero
- [ ] M2 Create & launch one emulator end-to-end
- [ ] M3 Registry & reliable tracking
- [ ] M4 Profiles: export / import / recreate
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

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
