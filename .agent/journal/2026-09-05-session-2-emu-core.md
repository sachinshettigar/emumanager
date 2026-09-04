# 2026-09-05 — session 2 (Claude Code) — emu-core ports & models

## Worked on

- Task(s): `0002` (emu-core ports and models — no impls)
- Milestone: `M0`

## Changed

- `crates/emu-core/Cargo.toml` — deps: `serde`, `async-trait`, `time` (features `std`,
  `macros`, `parsing`, `formatting`, `serde-well-known`), `url` (`serde`), `sha2`. Dev-deps:
  self `features=["testing"]`, `serde_json`, `tokio` (`rt`, `macros`).
- `crates/emu-core/src/lib.rs` — now just module wiring + re-exports.
- `crates/emu-core/src/error.rs` — `CoreError` (9 variants) + stable `code()` + `parse()` /
  `invalid()` helpers.
- `crates/emu-core/src/model/{mod,device,image,emulator,host,job,plan,profile}.rs` — the domain
  entities from `docs/context/domain-model.md`, `serde`-ready.
- `crates/emu-core/src/ports.rs` — `Command`/`Output`, `ChildProcess`, `ProcessRunner`,
  `Downloader`, `HostProbe`, `Clock`, `Fs`, `pct_progress`.
- `crates/emu-core/src/provider.rs` — `Provider` trait, `LaunchOpts`, `RunningHandle`.
- `crates/emu-core/src/testing/mod.rs` — `FakeProcessRunner`, `FakeChild`, `FakeDownloader`,
  `FakeClock`, `InMemoryFs` (feature `testing`).
- `clippy.toml` — `doc-valid-idents` += `MiB`/`GiB`/`KiB`/`TiB`.
- Task file + `.agent/state.json` + `PROGRESS.md` + this journal.

## State now

- `cargo test -p emu-core --all-features` ✅ 39 · `cargo test -p emu-core` ✅ 39 (self dev-dep
  turns on `testing`) · `cargo test --workspace --all-features` ✅ · `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` ✅ · `cargo fmt --all --check` ✅ ·
  `scripts/emu-core-no-tauri.sh` ✅ · `node scripts/progress-check.mjs` ✅
- Frontend untouched, still green.
- `just validate`: still pre-0007 (missing lint CLIs; `cargo sqlx prepare --check` needs 0005).
- Tasks moved: `0002` todo → doing → done.
- `lastValidatedCommit`: null.

## Next action

Task `0004` — tauri-specta. Add a `ping` `#[tauri::command]` in `src-tauri`, wire
`tauri-specta` to emit `src/lib/bindings.ts`, make `just bindings` regenerate it, and use it
from the frontend. **Also add `#[derive(specta::Type)]`** to the `emu-core` DTOs that cross IPC
(list in `.agent/tasks/0002-*.md` → "Decisions / deviations"). Pin `tauri-specta` + `specta`
versions together. Fix the `justfile` `bindings` recipe if the test name differs from
`export_bindings` in `emumanager_lib`.

## Gotchas / notes for the next agent

- `emu-core` must stay `tauri`-free. `specta` is fine to add there (it's not `tauri`); do NOT
  add `tauri-specta` to `emu-core` — that goes in `src-tauri`.
- serde tag strings for `ImageType` / `Abi` are per-variant `#[serde(rename)]` equal to
  `.tag()`. If you add a variant, add both.
- Internally-tagged enums need `rename_all_fields = "camelCase"` (already on `EmulatorSource`,
  `Verdict`, `RequirementStatus`).
- `EmuProfile` deliberately mirrors the JSON Schema, not the domain types; keep it that way and
  reconcile the two documented naming mismatches in M4, not here.
- `time` `rfc3339` serde helpers need the `serde-well-known` feature — already enabled. Adding
  a timestamp field? Use `#[serde(with = "time::serde::rfc3339")]` (or `::option`).
- Tests use `#[tokio::test]` with a current-thread runtime; `tokio` is dev-only. Don't let it
  leak into `[dependencies]`.
- `cargo test -p emu-core` pulls a self dev-dependency with `features=["testing"]`. If that
  ever causes a resolver cycle warning, drop it and change the task's validate to
  `--all-features` (state.json already uses `--all-features`).
