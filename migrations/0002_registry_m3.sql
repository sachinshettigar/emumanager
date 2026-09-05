-- M3 registry schema (task 0018). Append-only: this file grows the M0-minimal
-- `emulators` table into the full tracked-emulator record and adds `host_snapshots`.
-- Never edit 0001; never edit this file once it has shipped — add 0003 instead.
--
-- Compound values (Hardware, EmulatorSource, tag list) are stored as JSON text so
-- adding a field to those structs later needs no new migration. Every added column
-- has a safe default so this applies cleanly to a database that already has rows
-- from 0001.

ALTER TABLE emulators ADD COLUMN device_profile_id TEXT NOT NULL DEFAULT '';
ALTER TABLE emulators ADD COLUMN image_coord       TEXT NOT NULL DEFAULT '';
ALTER TABLE emulators ADD COLUMN hardware_json     TEXT NOT NULL DEFAULT '{"ramMb":2048,"storageMb":6144,"graphics":"auto","snapshots":true,"coldBoot":false,"deviceFrame":true,"dpiOverride":null}';
ALTER TABLE emulators ADD COLUMN source_json       TEXT NOT NULL DEFAULT '{"kind":"manual","discovered":false}';
ALTER TABLE emulators ADD COLUMN tags_json         TEXT NOT NULL DEFAULT '[]';
ALTER TABLE emulators ADD COLUMN notes             TEXT NOT NULL DEFAULT '';

-- Live-run columns, refreshed by reconcile()/launch. `last_state` is the last
-- known lifecycle state ('stopped' | 'booting' | 'running' | 'error').
ALTER TABLE emulators ADD COLUMN last_state  TEXT NOT NULL DEFAULT 'stopped';
ALTER TABLE emulators ADD COLUMN adb_serial  TEXT;
ALTER TABLE emulators ADD COLUMN grpc_port   INTEGER;
ALTER TABLE emulators ADD COLUMN pid         INTEGER;
ALTER TABLE emulators ADD COLUMN launched_at TEXT;

ALTER TABLE emulators ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

-- One row per host-readiness probe (M5 fills these in; the table exists now so the
-- schema is complete). `report_json` is a serialized HostReport.
CREATE TABLE host_snapshots (
    id          TEXT PRIMARY KEY,
    captured_at TEXT NOT NULL,
    report_json TEXT NOT NULL
);
