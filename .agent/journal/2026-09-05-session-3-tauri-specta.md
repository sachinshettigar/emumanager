# 2026-09-05 — session 3 (Claude Code) — tauri-specta IPC seam

## Worked on

- Task(s): `0004` (tauri-specta wired: ping command + generated bindings)
- Milestone: `M0`

## Changed

- `src-tauri/Cargo.toml` — deps: `specta = "=2.0.0-rc.25"` (features `derive`, `serde_json`),
  `specta-typescript = "=0.0.12"`, `tauri-specta = "=2.0.0-rc.25"` (features `derive`,
  `typescript`). Pinned with `=` — v2 is an RC.
- `src-tauri/src/ipc_error.rs` (new) — `IpcError { code, message, details }`, `serde` +
  `specta::Type`, `Display`/`Error`, `impl From<emu_core::CoreError>` (`code` = `CoreError::code()`).
  `details` field carries `#[specta(type = specta_typescript::Unknown)]`. 3 tests.
- `src-tauri/src/commands.rs` (new) — `Pong { message, version }` + `#[tauri::command]
  #[specta::specta] ping(name: String) -> Result<Pong, IpcError>` (blank name → `invalid`).
  Module-level `#![allow(clippy::needless_pass_by_value)]` (command args are owned). 2 tests.
- `src-tauri/src/lib.rs` — `specta_builder()` (single source of truth), wired into `run()` via
  `invoke_handler` + `mount_events`; `#[cfg(test)] mod export` with `export_bindings` writing
  `../src/lib/bindings.ts` with an `@generated` / `eslint-disable` header.
- `src/lib/bindings.ts` (generated — do not hand-edit).
- `src/lib/ipc.ts` (new) — `IpcCallError extends Error` (wraps `IpcError` on `.ipc`), `ping()`
  unwrapping the `{status}` envelope, `usePing()` TanStack Query hook.
- `src/lib/ipc.test.tsx`, `src/routes/Dashboard.test.tsx` (new) — mock `./bindings`, assert the
  hook + the rendered "from vX.Y.Z" line. 4 tests.
- `src/main.tsx` — `QueryClientProvider` (retry off, no refetch-on-focus).
- `src/routes/Dashboard.tsx` — `<BackendStatus>` line ("pong, EmuManager from v0.1.0").
- `src/test/setup.ts` — `vi.mock("@tauri-apps/api/core")` → rejecting `invoke` by default.
- `src/test/renderRoute.tsx` — wrap the app in a `QueryClient`.
- `package.json` — `+@tanstack/react-query 5.102.8`, `+@tauri-apps/api 2.11.1`.
- `justfile` — `bindings` recipe de-stubbed: runs the `export::export_bindings` test.
- `eslint.config.js` + `.prettierignore` + `vitest.config.ts` — exclude `src/lib/bindings.ts`.
- `clippy.toml` — `doc-valid-idents` += `TypeScript`, `JavaScript`, `JSON`.
- Task file `0004`, `.agent/state.json`, `PROGRESS.md`, `MILESTONES.md`, this journal.

## State now

- `cargo test -p emumanager` ✅ 6 · `cargo test --workspace --all-features` ✅ (39 emu-core + 6) ·
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅ ·
  `cargo fmt --all --check` ✅ · `scripts/emu-core-no-tauri.sh` ✅
- `pnpm typecheck` ✅ · `pnpm lint` ✅ · `pnpm test` ✅ 11 · `pnpm format:check` ✅ ·
  `just check-fast` ✅
- `just bindings` then `git diff --exit-code src/lib/bindings.ts` ✅ (idempotent)
- `node scripts/progress-check.mjs` ✅
- `just validate`: still pre-0007 (missing lint CLIs; `cargo sqlx prepare --check` needs 0005).
- Tasks moved: `0004` todo → doing → review.
- `lastValidatedCommit`: null.

## Next action

Task `0005` — sqlx + SQLite. Add `sqlx` (runtime-tokio, sqlite, macros) to a crate that owns
the DB (likely `emu-core` behind a feature, or a new `emu-db` — check the task file), create
`migrations/0001_init.sql` for the registry tables from `docs/architecture.md`, run
`cargo sqlx prepare --workspace`, commit `.sqlx/`. Validate: `cargo sqlx prepare --check
--workspace`. `sqlx-cli` may need installing (`cargo install sqlx-cli --no-default-features
--features sqlite,rustls`).

## Gotchas / notes for the next agent

- **The specta triple is pinned with `=`.** Bump `tauri-specta` / `specta` /
  `specta-typescript` in one commit or the build breaks — `tauri-specta` pins `specta` with `=`
  internally.
- **`serde_json::Value` cannot be exported by specta** (it inlines `Number`, which holds
  `i64`/`u64`; specta forbids BigInt-lossy types). Any future DTO with a `serde_json::Value`
  field needs `#[specta(type = specta_typescript::Unknown)]` (or `Any`) on it.
- **`just bindings` = a Rust test**, not a build-script step. It runs
  `cargo test -p emumanager --lib export::export_bindings -- --exact` and writes
  `src/lib/bindings.ts`. Add a command? Register it in `collect_commands![...]` in
  `src-tauri/src/lib.rs`, run `just bindings`, commit the diff.
- **`emu-core` is still `tauri`-free and has no `specta` dep yet.** The crate-wide
  `#[derive(specta::Type)]` pass is deferred to the first M1 command that returns an `emu-core`
  type. When it lands: add `specta` (NOT `tauri-specta`) to `emu-core`, derive on the DTOs
  listed in `.agent/tasks/0002-*.md` → "Decisions / deviations", run `just bindings`.
- **`@tauri-apps/api/core` is mocked globally in `src/test/setup.ts`** to reject. A component
  test that needs a real reply must `vi.mock("../lib/bindings", ...)` (or `"./bindings"`) itself
  — see `src/routes/Dashboard.test.tsx`.
- **Known wart:** `pnpm test` prints a React Router v7 `startTransition` future-flag warning
  from `Sidebar.test.tsx` / `routes.test.tsx`. Tests pass; not chased here.
- Shell CWD persists between `Bash` calls in this harness — a stray `cd src-tauri` earlier made
  `mkdir -p src/lib` land in `src-tauri/src/lib`. Use absolute paths.
