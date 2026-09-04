# Playbook: what to test and where

See `docs/testing-and-validation.md` for the full matrix. This is the "where does my test go"
guide.

## Decision table

| You changed… | Test goes… | Style |
| --- | --- | --- |
| Logic in `emu-core` (resolve, plan, reconcile, error codes) | unit test next to the module | fake ports (`emu-core/src/testing`); `insta` for structured output |
| A parser in `emu-android` (sdkmanager/avdmanager/adb output) | `crates/emu-android/tests/` against a fixture | capture real `--help`/output into `tests/fixtures/<tool>/<case>.txt`, cite the command in a comment |
| Host detection in `emu-host` | unit test with a fake probe / recorded inputs | table test per OS/arch |
| An IPC command (`src-tauri`) | Rust test for input validation + `CoreError`→`IpcError` mapping | fake `App` state |
| A React component | colocated `*.test.tsx` | Vitest + Testing Library; mock `src/lib/bindings.ts` |
| A query hook (`src/lib/ipc.ts`) | colocated test | mock bindings module, assert loading/error/data states |
| The `.emuprofile` schema | `schemas/emuprofile/fixtures/{valid,invalid}/*.json` | add a fixture; `validate-schema.mjs` picks it up |
| DB schema / query | `emu-core/tests/` with a `tempdir` registry | run migrations, exercise the query |

## Rules

- **No network, no real Android binary** in anything `just validate` runs. Those live behind
  `#[ignore]` + `just test-integration` (nightly CI only).
- A bug fix starts with a failing test that reproduces it.
- Prefer one clear assertion of behavior over many brittle ones on internals.
- Coverage thresholds are gates, not goals — see the per-milestone number in `MILESTONES.md`. Add
  tests because the behavior matters, not to move the number.
- Name tests for the behavior: `resolves_needs_download_when_image_absent`, not `test_resolve_2`.
