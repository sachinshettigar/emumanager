# 2026-09-05 — session 6 (Claude Code)

## Worked on

- Task(s): `0012` (done)
- Milestone: `M1`

## Changed

- `crates/emu-core/src/toolchain/{mod,installed_state,bootstrap}.rs` (new) —
  `InstalledState::scan` (app-managed + system-SDK detection) and `bootstrap()` (download +
  extract `cmdline-tools`, accept licenses, `sdkmanager` installs the rest).
- `crates/emu-core/src/lib.rs` — `pub mod toolchain;`.
- `crates/emu-core/Cargo.toml` — added `zip` (archive extraction), `sha1` (catalog SHA-1
  verification), `tokio` (direct dep, `rt` feature, for clarity — already transitive via sqlx);
  dev-deps gained fuller `tokio` features, `reqwest`, and `emu-android` (for the integration test).
- `crates/emu-core/tests/toolchain_bootstrap.rs` (new) — real, `#[ignore]`d end-to-end test with
  test-local real port impls (can't depend on `src-tauri`). **Actually run**, not just written.
- `docs/adr/0006-require-system-jdk.md` (new) — the bundled-JRE-vs-system-JDK decision.
- `docs/spec.md` — §8's open question now points at the ADR instead of dangling.
- `scripts/progress-check.mjs` — relaxed the "one milestone in_progress" rule to allow a
  contiguous run ending at `currentMilestone` (M0 stays open on the externally-blocked CI task
  while M1 is worked).
- `.agent/tasks/0012-toolchain-bootstrap.md` → `done`, full Notes filled in.
- `.agent/state.json`, `MILESTONES.md`, `PROGRESS.md` updated.

## State now

- `just validate`: **pass**
- `just progress`: pass (after the relaxation above)
- Tasks moved: `0012` todo→done
- `lastValidatedCommit` in state.json: set to this session's commit after pushing
- Real `#[ignore]`d integration test run for real: downloaded actual `cmdline-tools` +
  `platform-tools` from Google, `sdkmanager --version` succeeded, ~35s, 2026-09-05.

## Next action

Task `0013` — the last M1 task: wire the Dependencies screen to real state. New `tauri-specta`
commands wrapping `installed_state::scan` and `bootstrap` (with job/progress events for the
frontend), `just bindings`, replace the screen's static placeholder content with real component
rows (installed/missing, where installed, sizes/versions from the catalog). Once done, M1 is
functionally complete except the deliberately-deferred download queueing/pause/cancel.

## Gotchas / notes for the next agent

- `bootstrap()`'s signature grew beyond the task file's illustrative `bootstrap(data_dir,
  components, ports)`: it's `bootstrap(data_dir, wanted, state, os, ports, job)`. `state` comes
  from calling `installed_state::scan` yourself first — `bootstrap` doesn't scan on its own, so a
  caller that skips this and passes a stale/empty `InstalledState` will happily re-download
  things that are actually already there.
- `toolchain::binary_path(sdk_root, id, os)` is the one true way to turn a `ComponentLocation`
  into an actual binary path — task 0013 and M2's `create`/`launch` should use it rather than
  re-deriving `cmdline-tools/latest/bin/sdkmanager`-style paths by hand.
- `LICENSE_ACCEPT_COUNT = 50` in `bootstrap.rs` is a deliberately generous bound based on a real
  capture (7 licenses, 2026-09-05) — if a much-larger real license list ever gets reported, that's
  the constant to revisit, not evidence of a bug.
- The archive-extraction executable-bit fix (`chmod -R +x` after extraction) only runs on
  non-Windows (`cfg!(windows)` guard) — if `NativeFs`/`Fs` ever grows a real permission-mode
  parameter, this whole `make_bin_executable` step becomes unnecessary and should be removed
  rather than left as redundant belt-and-braces.
- CI is still blocked on GitHub billing (Settings → Billing & plans) — see session-4's journal
  entry. Nothing to do there until a human fixes it.
