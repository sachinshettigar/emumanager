-- M4 (task 0023): saved `.emuprofile` recipes. `migrations/0001_init.sql`'s `profiles` table only
-- had (name, source_path, created_at); store the recipe JSON body and an optional description.
-- Append-only: never edit 0001/0002; add 0004 for the next change.

ALTER TABLE profiles ADD COLUMN json        TEXT NOT NULL DEFAULT '{}';
ALTER TABLE profiles ADD COLUMN description TEXT NOT NULL DEFAULT '';
