# .emuprofile schema

`v1.schema.json` is the JSON Schema (draft-07) for a `.emuprofile` recipe. See
[ADR 0004](../../docs/adr/0004-recipe-only-profile-sharing.md) for why recipes carry no binaries.

- The Rust `EmuProfile` struct in `emu-core` must stay in sync with this file; a test asserts it
  (M4 task).
- `fixtures/valid/*.json` — must validate. `fixtures/invalid/*.json` — must fail, each exercising
  one rule.
- `node ../../scripts/validate-schema.mjs` checks the schema is itself valid and every fixture is
  in the right folder. Part of `just validate` and `schema.yml`.

To evolve the format, add `v2.schema.json` (don't mutate v1) and a migration in the profile
engine; `schemaVersion` gates which schema applies.
