# Testing & validation

`just validate` is the gate. It must be green before a task is `done` and on every
commit to `main` (`AGENTS.md` §3, §6). `just check-fast` is the inner-loop subset.

## How to run

| Command | What |
| --- | --- |
| `just check-fast` | fmt, clippy, typecheck, lint, `emu-core` unit tests, progress-check. Seconds. |
| `just validate` | The full gate below. Runs every step, collects failures, reports them all at the end. |
| `just test` / `just test-rust` / `just test-web` | Just the test suites. |
| `bash scripts/validate.sh` | Same as `just validate` (the recipe just calls this). |

To run one check, invoke it directly — e.g. `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `pnpm test`, `pnpm exec knip`.

## The matrix

Core checks always run and must pass. Optional external tools run when installed
and otherwise print `(skip: <tool> …)`; a skip never fails the gate. `just setup`
and CI (task 0009) install the optional tools.

| Step | Tool | Catches | Always on? |
| --- | --- | --- | --- |
| progress files | `scripts/progress-check.mjs` | `.agent/state.json` ↔ `MILESTONES.md` ↔ task files drift | yes |
| emuprofile schema | `scripts/validate-schema.mjs` (`ajv`) | `.emuprofile` schema self-validity + every fixture | yes (full check needs `ajv`, a dev-dep) |
| `emu-core` purity | `scripts/emu-core-no-tauri.sh` | `emu-core` gaining a `tauri` dependency | yes |
| rust fmt | `cargo fmt --check` | formatting | yes |
| rust clippy | `cargo clippy --all-targets --all-features -D warnings` | lints, `pedantic` group | yes |
| rust tests | `cargo nextest` or `cargo test` (`--workspace --all-features`) | unit + integration tests | yes |
| rust coverage | `cargo llvm-cov` | line coverage vs the milestone threshold | from M1 (M0 gate is n/a) |
| dependency policy | `cargo deny check` | licenses, advisories, banned/duplicate crates (`deny.toml`) | optional |
| unused deps | `cargo machete` | declared-but-unused crates | optional |
| sqlx offline data | `cargo sqlx prepare --check` | `.sqlx/` stale vs the code | from M3 (no `query!` macros yet) |
| ipc bindings | `just bindings` + `git diff --exit-code` | `src/lib/bindings.ts` stale vs the Rust commands | yes |
| web typecheck | `tsc -b` | TypeScript errors (`strict`) | yes (needs `node_modules`) |
| web lint | `eslint` | `typescript-eslint` strict-type-checked, react-hooks | yes |
| web format | `prettier --check` | formatting | yes |
| web tests | `vitest run` | component/unit tests | yes |
| unused files/exports | `knip` (dev-dep, `knip.json`) | dead code, unused deps, unlisted deps | yes |
| markdown | `markdownlint-cli2` (dev-dep, `.markdownlint-cli2.yaml`) | broken markdown structure | yes |
| links | `lychee --offline` (`lychee.toml`) | dangling relative links between repo files | optional |
| typos | `typos` (`_typos.toml`) | spelling in code + docs | optional |
| workflows | `actionlint` | GitHub Actions syntax/expression errors | optional |
| secrets | `gitleaks detect` (`.gitleaks.toml`) | committed credentials | optional |

## Git hooks (lefthook)

`lefthook` is a dev-dependency; `just setup` (or `just hooks`) runs `lefthook
install`. CI never triggers these — it calls `just validate` directly, and the
`pre-push` hook no-ops when `CI` is set.

| Hook | Runs | Skip |
| --- | --- | --- |
| `pre-commit` | `cargo fmt` + `prettier --write` on staged files, `just bindings` (when `*.rs`/`*.toml` staged) + re-stage `src/lib/bindings.ts`, `just db-prepare` (only when `.sqlx/` + `cargo-sqlx` exist), `gitleaks protect` (when installed), `just progress` | `LEFTHOOK=0 git commit …` |
| `commit-msg` | Conventional Commits regex (`feat\|fix\|chore\|docs\|refactor\|test\|perf\|build\|ci`) | `LEFTHOOK=0` |
| `pre-push` | `just validate` | `LEFTHOOK=0 git push`; auto-skipped in CI |

## Notes

- **Not fail-fast by design.** One `just validate` run surfaces every problem, so
  the aggregator scripts use `set -uo pipefail` (not `-e`) and tally failures.
- **`SQLX_OFFLINE`** is irrelevant today: the registry uses runtime `query_as`,
  not the checked `query!` macros, so nothing reads `DATABASE_URL`. When M3 adopts
  `query!`, add `.sqlx/` (via `just db-prepare`) and drop the `[[ -d .sqlx ]]`
  guards in `scripts/validate.sh` and `lefthook.yml`.
- **Real-binary tests** (`adb`, `sdkmanager`, …) live behind `#[ignore]` /
  `just test-integration` and are **not** part of `just validate` (`AGENTS.md` §6.4).
- **E2E** (`just e2e`, task 0030) builds the debug app and drives it through
  `tauri-driver` + WebdriverIO (`e2e/`, self-contained — its own `package.json`
  and `node_modules`, so it never affects `just validate`). One smoke spec today
  (`e2e/specs/smoke.e2e.ts`: app launches, Dashboard + sidebar render, the typed
  IPC seam reaches a `pong`). Linux/Windows only — `tauri-driver` has no macOS
  support, so on a Mac `just e2e` prints a pointer to
  `docs/playbooks/macos-e2e-checklist.md` and exits 0. Runs in CI's `e2e` job and
  the nightly (both gated on the GitHub Actions billing block, same as the rest).
