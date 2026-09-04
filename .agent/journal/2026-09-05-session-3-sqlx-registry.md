# 2026-09-05 — session 3 (Claude Code) — sqlx registry bootstrap

## Worked on

- Task(s): `0005` (sqlx + initial migration + registry open)
- Milestone: `M0`
- Also: added `@tauri-apps/cli` so `just dev` runs (commit `b496976`).

## Changed

- `crates/emu-core/Cargo.toml` — `sqlx = "0.8"` (`default-features = false`, features
  `runtime-tokio`, `sqlite`, `migrate`, `macros`). dev-deps: `tempfile`, `sqlx` (for `query_as`
  in the test), `tokio` += `rt-multi-thread`.
- `crates/emu-core/src/registry/{mod.rs,open.rs}` (new) — `Registry { pool: SqlitePool }`,
  `Registry::open(&Path)` (create dir, `create_if_missing`, `foreign_keys`, WAL,
  `sqlx::migrate!("../../migrations")`), `Registry::pool()`.
- `crates/emu-core/src/lib.rs` — `pub mod registry;`.
- `crates/emu-core/src/error.rs` — `CoreError::Db { detail }` → code `db_error` (+ test line).
- `migrations/0001_init.sql` (new) — `emulators`, `images`, `profiles`, `jobs`; `TEXT` PKs,
  `created_at TEXT` default via `strftime`.
- `crates/emu-core/tests/registry_open.rs` (new) — tempdir open, assert file + `SELECT COUNT(*)`,
  re-open idempotent.
- `justfile` — `db-migrate` → forward-only `cargo sqlx migrate add`; `db-prepare` comment.
- `scripts/validate.sh` — `sqlx prepare --check` step gated on `[[ -d .sqlx ]]`.
- `clippy.toml` — `doc-valid-idents` += `SQLite`, `WAL`.
- `package.json` — `+@tauri-apps/cli 2.11.4` (devDep).
- Task file `0005`, `.agent/state.json`, `PROGRESS.md`, `MILESTONES.md`, this journal.

## State now

- `SQLX_OFFLINE=true cargo test -p emu-core registry_open` ✅ 1 ·
  `cargo test --workspace --all-features` ✅ (39 unit + 1 registry + 6 emumanager) ·
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅ ·
  `cargo fmt --all --check` ✅ · `scripts/emu-core-no-tauri.sh` ✅ ·
  `node scripts/progress-check.mjs` ✅
- `just dev` launches the app window (verified via `pnpm tauri dev`).
- Frontend web tests unchanged (11).
- `just validate`: still pre-0007 (missing lint CLIs).
- Tasks moved: `0005` todo → done.
- `lastValidatedCommit`: null.

## Next action

Task `0007` — wire `just validate` end to end. Install / gate the remaining static tools
(`cargo-deny`, `cargo-machete`, `actionlint`, `gitleaks`, `markdownlint-cli2`, `knip`), make
every step either run or cleanly skip, and set `lastValidatedCommit` in `.agent/state.json`
once it passes green. `scripts/validate.sh` already has the skeleton with `have`-guards.

## Gotchas / notes for the next agent

- **`sqlx::migrate!` needs the `macros` feature** (not just `migrate`) — it is a proc-macro.
  That is the *only* reason `macros` is on; we use runtime `query_as`, not `query!`, so there is
  no `DATABASE_URL` and no `.sqlx/` cache anywhere. Adopt `query!` + `cargo sqlx prepare` +
  commit `.sqlx/` in M3 when the real schema/queries land, and drop the `[[ -d .sqlx ]]` guards.
- **`sqlx sqlite` bundles SQLite** (compiles it) — first build is slower but CI needs no
  `libsqlite3-dev`.
- **`migrate!` path is relative to `CARGO_MANIFEST_DIR`** = `crates/emu-core`, hence
  `"../../migrations"`. Editing/adding a migration triggers a recompile of `emu-core`.
- **`lefthook.yml` line 15-16** (`db-prepare: just db-prepare && git add .sqlx`) will fail once
  lefthook is installed (task 0008) — gate it on `[[ -d .sqlx ]]` like `validate.sh`.
- **`Registry` is not opened from `src-tauri` yet** — do it in `run()` (or a command) in M3
  when something first reads/writes the DB; needs the data-dir path from Tauri's path resolver.
- The task's validate command filters by test *name*, so the test function is named
  `registry_open_*` (not `opens_*`) to match `cargo test -p emu-core registry_open`.
