# Testing & validation

Everything `just validate` runs, what each tool catches, and how to run one in isolation. This is
the gate: green here == green in CI. No step touches the network or spawns a real Android binary.

## The matrix

| # | Step | Command | Catches |
| --- | --- | --- | --- |
| 1 | Rust format | `cargo fmt --check` | style drift |
| 2 | Rust lint | `cargo clippy --workspace -- -D warnings` (pedantic on, targeted allows in `clippy.toml`) | bugs, foot-guns, non-idiomatic code |
| 3 | Core purity | `scripts/emu-core-no-tauri.sh` | `emu-core` gaining a `tauri` dep |
| 4 | Rust tests | `cargo nextest run --workspace` | logic regressions; parser fixtures; registry behavior |
| 5 | Rust coverage | `cargo llvm-cov --workspace --fail-under-lines <gate>` | untested code below the milestone gate |
| 6 | SQLx offline | `cargo sqlx prepare --check --workspace` | queries changed without refreshing `.sqlx/` |
| 7 | Dependency policy | `cargo deny check` | disallowed licenses, security advisories, banned/duplicate crates |
| 8 | Unused Rust deps | `cargo machete` | dead dependencies |
| 9 | IPC bindings fresh | `just bindings && git diff --exit-code src/lib/bindings.ts` | hand-edited or stale bindings |
| 10 | TS types | `pnpm typecheck` (`tsc --noEmit`, strict) | type errors |
| 11 | TS lint | `pnpm lint` (`typescript-eslint` strict-type-checked + react-hooks + import) | bugs, hook misuse, bad imports |
| 12 | TS format | `pnpm format:check` (Prettier) | style drift |
| 13 | Frontend tests | `pnpm test` (Vitest + Testing Library) + coverage gate | component/hook regressions |
| 14 | Unused TS | `pnpm knip` | dead files / exports / deps |
| 15 | Schema | `node scripts/validate-schema.mjs` (ajv) | `.emuprofile` schema invalid; any fixture not matching its folder |
| 16 | Markdown | `markdownlint-cli2` | broken doc formatting |
| 17 | Links | `lychee --offline .` | dead relative links in docs |
| 18 | Spelling | `typos` | typos in code + docs (allowlist in `_typos.toml`) |
| 19 | Workflows | `actionlint` | broken GitHub Actions YAML |
| 20 | Secrets | `gitleaks detect` | committed keys/tokens |
| 21 | Progress files | `just progress` (`scripts/progress-check.mjs`) | `.agent/state.json` vs task files vs `MILESTONES.md` drift |

## Not in `just validate` (heavier, run elsewhere)

| Step | Command | When |
| --- | --- | --- |
| Integration | `just test-integration` | nightly CI — real `sdkmanager`/`avdmanager` in a scratch SDK dir |
| E2E (full) | `just e2e` with `tauri-driver` | per-milestone DoD + release; Linux + Windows only |
| E2E (UI) | Playwright against `vite dev` | per PR touching UI; all OSes incl. macOS |
| Bundle build | `pnpm tauri build --debug` | CI matrix, as an artifact |
| Supply chain | `osv-scanner`, `cargo audit` | nightly + Dependabot/Renovate |

## Coverage gates

Per milestone, in `MILESTONES.md` (start 60%, ratchet to 75% by v1.0). Gates are floors, not
targets — see `docs/playbooks/write-tests.md`.

## Running one step

Every command above runs standalone. The umbrella scripts are `scripts/validate.sh` (all) and
`scripts/check-fast.sh` (1, 3, 10, 11, and affected-crate tests). If a tool is missing,
`validate.sh` prints which and how to install it (all are dev-deps or installed by `just setup`).

## Adding a new check

Add it to `scripts/validate.sh`, this table, and (if it needs a binary) `scripts/setup.sh` and
the CI composite action. Keep total CI runtime under ~8 min warm.
