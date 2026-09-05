---
id: "0018"
title: "Full registry schema + migration + typed Registry API"
milestone: "M3"
status: "review"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

Grow the M0-minimal `emulators` table into the real M3 schema (image coord, device profile,
hardware, source, live-run columns, timestamps) plus a `host_snapshots` table, behind a **typed**
`Registry` API (`EmulatorRow` struct in / out) instead of the current `(String, String, String)`
tuples. Nothing new spawns a process or reconciles yet — this is the storage layer the rest of M3
builds on.

## Context / links

- Architecture: `docs/architecture.md#3-key-abstractions` (Registry: tables + `reconcile()`)
- Milestones: `MILESTONES.md` M3 bullet 1
- Existing: `migrations/0001_init.sql` (append-only — never edit it), `crates/emu-core/src/registry/open.rs`
- `crates/emu-core/src/model/emulator.rs` — `Emulator` / `EmulatorSource` / `Hardware` / `RunState`
  already model everything the columns need; this task persists them.

## Scope — files this task may touch

- `migrations/0002_registry_m3.sql` (new — append-only migration)
- `crates/emu-core/src/registry/mod.rs`, `crates/emu-core/src/registry/open.rs`
- `crates/emu-core/src/registry/row.rs` (new — `EmulatorRow` + `HostSnapshotRow` typed structs, mapping to/from `sqlx::Row`)
- `crates/emu-core/src/registry/tests.rs` or inline `#[cfg(test)]` (round-trip tests)
- `crates/emu-android/src/provider.rs` — only where `create` / `tracked_states` call the changed `Registry` methods
- `src-tauri/src/commands/emulator.rs` — only where `list_emulators` maps rows (keep it compiling)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `migrations/0002_registry_m3.sql` grows `emulators` with `ALTER TABLE ADD COLUMN` (one per
      column, each with a safe default so it applies to a `0001`-only DB): `device_profile_id`,
      `image_coord`, `hardware_json`, `source_json`, `tags_json`, `notes`, `last_state` (`'stopped'`
      default), `adb_serial`, `grpc_port`, `pid`, `launched_at`, `updated_at`. **Compound values
      (`Hardware`, `EmulatorSource`, tags) are one JSON column each**, not one SQL column per field
      — see Notes for why.
- [x] New table `host_snapshots (id TEXT PRIMARY KEY, captured_at TEXT NOT NULL, report_json TEXT NOT NULL)`
- [x] `EmulatorRow` struct (`registry/row.rs`): every column as a typed field — `Option<ImageCoord>`,
      `Hardware`, `EmulatorSource`, `RunState`, `Vec<String>` tags, `OffsetDateTime` timestamps,
      `Option<u16>` grpc_port, `Option<u32>` pid. `serde_json` for the `*_json` columns; `RunState`
      as its lowercase string; timestamps as RFC 3339 (SQLite `strftime` output already is).
      `HostSnapshotRow` too.
- [x] `Registry` gains: `upsert_emulator(&EmulatorRow)` (INSERT … ON CONFLICT(id) DO UPDATE, keeps
      `created_at`), `get_row`, `list_rows`, `set_run_fields`, `rename`, `set_hardware`,
      `delete_row -> bool`, `insert_host_snapshot(&HostSnapshotRow)`, `latest_host_snapshot()`.
- [x] Old `insert_emulator` / `get_emulator` / `list_emulators` **removed**; the two callers
      (`emu-android::provider` `create` / `launch` / `stop` / `tracked_states`) updated to the typed API.
- [x] `emu-android::AndroidProvider::create` writes a full `EmulatorRow` (source
      `Manual { discovered: false }`, `last_state = Stopped`); `tracked_states` reads typed rows.
- [x] Runtime `sqlx::query` only — **no `query!` macros / no `.sqlx/`** this task (see Notes). Did
      not block `just validate`.
- [x] Tests: migration applies + every new column round-trips through `upsert_emulator` → `get_row`;
      upsert preserves `created_at`; `set_run_fields` / `rename` / `set_hardware` / `delete_row` /
      `list_rows` ordering / duplicate-`avd_name` rejection / `host_snapshots` all covered (6 inline
      tests in `registry::open`).
- [x] `just check-fast` passes, then `just validate` green (117 rust tests, 22 web)
- [x] Docs updated (`MILESTONES.md` M3 bullet 1, architecture Registry note)

## Validate

```
cargo test -p emu-core -p emu-android --all-features
just validate
```

## Notes / findings

### Compound values are JSON columns, not one column per field

The task's first draft listed `ram_mb` / `storage_mb` / `graphics` / `snapshots` / `cold_boot` as
separate columns. Switched to a single `hardware_json` column holding the whole serialized
`Hardware` (and likewise `source_json`, `tags_json`). Reasons: (1) `Hardware` has 7 fields
(`device_frame`, `dpi_override` too) — per-column would either lose two of them or need two more
columns; (2) `EmulatorSource` is an enum with per-variant payloads (`FromProfile { profile_id }`,
`Imported { profile_id, origin_label }`) — awkward to normalise into columns; (3) adding a field to
any of those structs later then needs **no migration**. Cost: you can't `WHERE hardware...` in SQL,
which nothing needs — every consumer loads the whole row. The migration's `hardware_json` default
is `Hardware::default()` serialized verbatim (camelCase, `graphics:"auto"`); a test asserts the
round trip so a drift in the default is caught.

### `.sqlx/` / `query!` macros — still deferred, now to M4

`migrations/0001`'s comment and `MILESTONES.md` M0 both said the offline cache lands "in M3". It
doesn't, here: runtime `sqlx::query` + manual `SqliteRow::try_get` mapping works, `just validate`
is green without a `DATABASE_URL` or a committed `.sqlx/`, and adopting `query!` now would mean a
build-time DB or a checked-in cache for every query in the crate. Not worth it for this task's
value. Moved the pointer: `MILESTONES.md` M4 row + `crates/emu-core/Cargo.toml` comment now say M4.
If `query!` is never adopted that's fine too — runtime queries against a bundled SQLite are not a
correctness risk, only a "typo caught at compile time vs. first test run" convenience.

### Tuple API deleted, not shimmed

`insert_emulator` / `get_emulator` / `list_emulators` are gone. Only `emu-android::provider` called
them; its `create` now builds an `EmulatorRow`, and `launch` / `stop` / `tracked_states` read
`get_row` / `list_rows`. `src-tauri` never touched the registry directly (it goes through the
provider), so no command changed.

### No SQLite `ALTER TABLE` limitation hit

SQLite only allows `ADD COLUMN` (no `DROP` / `ALTER COLUMN`), which is all this migration does.
Every added column has a literal or `strftime(...)` default, so it applies to a table that already
has `0001` rows without a rewrite.

### `RunState` match is exhaustive inside the crate

`RunState` is `#[non_exhaustive]`, but that only constrains *downstream* crates — inside `emu-core`
the `run_state_str` match is exhaustive and a `_` arm is `unreachable_patterns`. Left it without one.
</content>
</invoke>
