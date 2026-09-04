# 2026-09-05 — session 3 (Claude Code) — `just validate` gate

## Worked on

- Task(s): `0007` (`just validate` wired end to end)
- Milestone: `M0`

## Changed

- `scripts/validate.sh` — rewritten: ordered steps, `step`/`run`/`have`/`skip` helpers,
  run-all/report-all, a final `validate: OK (N skipped)` / `FAILED` summary. Core checks always
  run; optional external tools skip with a message.
- `scripts/setup.sh` — cargo loop gained `typos-cli`; CLI-tools loop tries `brew install` for
  `actionlint`/`gitleaks`/`lychee`; notes that `markdownlint-cli2` + `knip` are dev-deps now.
- `package.json` — `+knip 5.39.2`, `+markdownlint-cli2 0.15.0`, `+ajv 8.20.0`;
  `-@testing-library/user-event` (unused, per knip).
- `knip.json` (new) — entry/project globs, ignore `src/lib/bindings.ts`, ignore
  `markdownlint-cli2` dep + `just`/`playwright` binaries.
- `src/lib/ipc.ts` — exports trimmed to `usePing` + `IpcCallError` (knip: `ping` and the
  `IpcError`/`Pong` re-exports were unused). `ping` is now a module-private helper.
- `.markdownlint-cli2.yaml` — MD022/MD028/MD031/MD032/MD036/MD040 disabled (fire only on
  pre-existing hand-written docs).
- `.agent/tasks/0003-*.md` — one real markdown fix (stray leading `+` list marker).
- `.gitleaks.toml`, `lychee.toml`, `.config/nextest.toml` (new) — minimal configs.
- `docs/testing-and-validation.md` (new) — how to run, the matrix, notes.
- Task file `0007`, `.agent/state.json` (+ `lastValidatedCommit`), `PROGRESS.md`,
  `MILESTONES.md`, this journal.

## State now

- `just validate` → **`validate: OK`** (core checks pass; 9 optional skips: nextest, deny,
  machete, llvm-cov, sqlx-check, typos, actionlint, gitleaks, lychee).
- `just check-fast` ✅ · `node scripts/progress-check.mjs` ✅
- Rust: `cargo test --workspace --all-features`, `clippy --all-targets --all-features -D
  warnings`, `fmt --check`, `emu-core-no-tauri.sh` ✅
- Web: `pnpm typecheck/lint/format:check/test` (11) ✅ · `knip` ✅ (exit 0) ·
  `markdownlint-cli2` ✅ (0 errors, 52 files)
- Schema check now runs in **ajv mode** (valid + invalid fixtures).
- Tasks moved: `0007` todo → done.
- `lastValidatedCommit`: the task-0007 commit (set in state.json).

## Next action

Task `0008` — lefthook hooks. `lefthook.yml` already exists; install lefthook, wire pre-commit
(fmt + `just bindings` + stage results; **gate the `db-prepare` step on `[[ -d .sqlx ]]`** —
it will fail otherwise, see task 0005/0007 notes) and pre-push (`just validate`). Verify with
`lefthook run pre-commit`.

## Gotchas / notes for the next agent

- **`just validate` is green *with skips*, by design.** A skip (tool absent) never fails it; a
  present tool that fails does. Don't "fix" the skips by making them errors — install the tools
  (`just setup`) or let CI do it (0009).
- **knip is a hard gate now.** Adding an export nothing imports, or a dep nothing uses, fails
  `just validate`. Either wire it up, delete it, or add to `knip.json` `ignore*` with a reason.
- **markdownlint is a hard gate** with a relaxed ruleset. If you re-enable MD022/032/031/etc.,
  do the docs-formatting pass in the same commit.
- **`.markdownlint-cli2.yaml`** detection in `validate.sh` uses `node_modules/.bin/markdownlint-cli2`
  (the `--help`/`--version` flags are parsed as globs by that tool, so they're useless for
  presence checks).
- `cargo llvm-cov` threshold is `--fail-under-lines 0` until M1 picks the real number
  (`MILESTONES.md` per-milestone coverage gates).
