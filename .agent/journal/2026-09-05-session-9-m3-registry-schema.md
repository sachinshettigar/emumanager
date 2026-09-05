# 2026-09-05 — session 9 (Claude Code)

## Worked on

- M3 scoped into tasks `0018`–`0021` (same granularity as M1/M2).
- Task `0018` (done, in review): full registry schema + typed `Registry` API.
- Milestone: `M3` (now `in_progress`).

## Changed

- `.agent/tasks/0018`–`0021` (new) — the four M3 task files.
- `migrations/0002_registry_m3.sql` (new) — `ALTER TABLE` grows `emulators` to the full tracked
  record (`device_profile_id`, `image_coord`, `hardware_json`, `source_json`, `tags_json`, `notes`,
  `last_state`, `adb_serial`, `grpc_port`, `pid`, `launched_at`, `updated_at`), adds `host_snapshots`.
- `crates/emu-core/src/registry/row.rs` (new) — `EmulatorRow` + `HostSnapshotRow` typed records,
  `SqliteRow` → struct via manual `try_get`; JSON columns via `serde_json`.
- `crates/emu-core/src/registry/open.rs` — rewritten: typed API (`upsert_emulator`, `get_row`,
  `list_rows`, `set_run_fields`, `rename`, `set_hardware`, `delete_row`, `insert_host_snapshot`,
  `latest_host_snapshot`); old tuple methods deleted; 6 inline tests.
- `crates/emu-core/src/registry/mod.rs` — `pub use row::{EmulatorRow, HostSnapshotRow}`.
- `crates/emu-core/Cargo.toml` — `serde_json` dep; `.sqlx` comment now says M4.
- `crates/emu-android/src/provider.rs` — `create` builds a full `EmulatorRow`; `launch` / `stop` /
  `tracked_states` read the typed rows; test helpers updated.
- `crates/emu-android/Cargo.toml` — `time` dep (one `now_utc()`).
- `MILESTONES.md` (M3 bullet 1 ticked, M0 `.sqlx` line → M4), `docs/architecture.md` (Registry
  note), `PROGRESS.md`, `.agent/state.json`.

## State now

- `just validate`: **pass** (117 rust tests, 22 web tests).
- `just progress`: pass (milestone M3, 21 tasks / 21 files).
- Tasks moved: `0018` todo→doing→review; `0019`–`0021` created as todo.
- `lastValidatedCommit` in state.json: set to this session's commit after pushing.

## Next action

Task `0019` — `AndroidProvider::reconcile()` + `delete()` + kill-safety + the M3 DoD property test.
`reconcile()`: parse `avdmanager list avd` (needs a real captured fixture — capture it the way
`0014`/`0015` did, or fall back to the documented shape and say so), then adopt on-disk AVDs with
no row (`EmulatorSource::Manual { discovered: true }`), flag rows whose AVD vanished
(`last_state = Error`, don't hard-delete), refresh `last_state` / `adb_serial` / `grpc_port` from
adb via the helpers `tracked_states` already uses, and reset a `Booting`/`Running` row whose `pid`
is set but not in `adb devices` back to `Stopped` (kill-safety). `delete(id, wipe)`:
`avdmanager delete avd -n <name>` + `delete_row`; `wipe` also removes the AVD data dir via `Fs`.

## Gotchas / notes for the next agent

- **Compound values are JSON columns** (`hardware_json` / `source_json` / `tags_json`), not one
  column per field. `EmulatorRow::from_sqlite_row` is the only place that (de)serializes them. Add
  a field to `Hardware` / `EmulatorSource` and nothing in the registry needs to change — but the
  migration's `hardware_json` DEFAULT literal is a frozen snapshot of `Hardware::default()`; if you
  change that default, the round-trip test in `registry::open` will catch it and you add an
  `0003` migration rather than editing `0002`.
- `RunState` is `#[non_exhaustive]` but the `run_state_str` match in `row.rs` is exhaustive
  *inside the crate* — no `_` arm (it'd be `unreachable_patterns`). A new `RunState` variant means
  updating both `run_state_str` and `parse_run_state`.
- `Registry` methods take `&str` ids, not `&EmulatorId` — callers pass `id.as_str()`.
- `set_run_fields` / `rename` / `set_hardware` all bump `updated_at` to `now_utc()` themselves;
  `upsert_emulator` takes whatever `updated_at` the row carries (so a full re-upsert should set it).
- `.sqlx/` + `query!` macros deferred to **M4** now (was "M3" in three places, all updated).
- No IPC surface changed this task — `bindings.ts` is a no-op regen. That changes in `0020`.
- CI still blocked on GitHub billing — nothing to do until a human fixes it.
