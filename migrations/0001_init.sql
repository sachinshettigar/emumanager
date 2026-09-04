-- Minimal registry bootstrap (M0). The full schema — extra columns, indexes,
-- and the job/event tables — lands in M3. Migrations are append-only: add a new
-- file, never edit this one.

CREATE TABLE emulators (
    id           TEXT PRIMARY KEY,
    avd_name     TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE images (
    coord      TEXT PRIMARY KEY,   -- e.g. system-images;android-34;google_apis;x86_64
    installed  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE profiles (
    name        TEXT PRIMARY KEY,
    source_path TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE jobs (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL,
    state      TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
