# 2026-09-05 — session 3 (Claude Code) — CI workflows

## Worked on

- Task(s): `0009` (CI: ci.yml matrix + schema.yml + emu-core-no-tauri check)
- Milestone: `M0`

## Changed

- `.github/actions/setup/action.yml` — **added the Linux Tauri v2 build deps that were
  entirely missing** (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libssl-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`, `build-essential`, `curl wget file`), guarded
  `if: runner.os == 'Linux'`. Swapped the from-source `cargo install cargo-nextest cargo-deny
  cargo-machete cargo-llvm-cov sqlx-cli` for `taiki-e/install-action@v2` (the four common ones)
  plus a `cargo install sqlx-cli typos-cli --locked || true` fallback for the two uncertain
  ones. Added an `actionlint` install step via `download-actionlint.bash`. Dropped the redundant
  `npm i -g markdownlint-cli2` (it's a project dev-dep now, task 0007).
- `.github/workflows/ci.yml` — added `timeout-minutes` (5 on `fast`, 20 on `gate`); otherwise
  unchanged (fast ubuntu smoke job → 3-OS `gate` job running `just validate` + `pnpm tauri build
  --debug` + artifact upload).
- `.github/workflows/nightly-integration.yml` — `e2e` step now checks for
  `playwright.config.{ts,js}` and skips with a message when absent (no suite exists yet).
- `.github/workflows/schema.yml` — unchanged, already correct (installs `ajv` standalone).
- `.github/dependabot.yml` (new) — cargo + npm + github-actions, weekly, patch-grouped.
- `docs/playbooks/milestone-review.md` — branch-protection step now names the exact required
  checks (`ci / fast`, `ci / gate (<os>)` ×3, `schema / emuprofile`) and flags it as a human (or
  explicitly-directed agent) action.
- Task file `0009` (status `review`, not yet `done`), `.agent/state.json`, `PROGRESS.md`,
  `MILESTONES.md`, this journal.

## State now

- `actionlint` (downloaded via `download-actionlint.bash` into `~/.local/bin` for this session)
  → **clean** on all 4 workflow files.
- `just validate` locally now also runs `actionlint` for real (was skipping before) — 8 optional
  skips (down from 9): nextest, llvm-cov, cargo-deny, cargo-machete, sqlx-check, typos, gitleaks,
  lychee.
- Pushed to `main` — **watching the live `ci.yml` run** (see Next action). Task `0009` marked
  `review` in both the task file and `.agent/state.json`; `M0` stays `in_progress` until that
  run is confirmed green.

## Next action

1. `gh run watch` (or `gh run list` / the Actions tab) on the push that includes this commit.
   Fix anything the live runners catch that local `actionlint`/dry-reading couldn't (composite
   action step ordering, an apt package rename on a newer Ubuntu image, a `taiki-e/install-action`
   tool name that doesn't exist, Windows `bash <(curl ...)` process substitution, etc.).
2. On green: flip task `0009` → `done` in the task file + `.agent/state.json`, tick the two
   remaining `MILESTONES.md` M0 boxes (CI green; lefthook was already ticked in 0008), set `M0`
   milestone `status: "done"`, bump `currentMilestone` to `"M1"`, update `lastValidatedCommit`.
3. Then: the M0 wrap-up the user asked for — a short "how the codebase is organized" learning
   doc — is due once all of M0 (not "all features") is closed out per the original
   session-start instruction ("generated at the end of all feature"). Confirm with the user
   whether that means end of M0 or end of the whole project before writing it; the safer
   reading given the instruction's placement in an M0-heavy session is end of M0, but re-check
   PROGRESS.md's Log for whether the user clarified this elsewhere first.
4. Start M1 (`docs/context/domain-model.md` + `MILESTONES.md` M1 DoD): the toolchain manager —
   real `sdkmanager`/`avdmanager` process wrapping in `emu-android`, first real `Downloader`,
   first commands that return `emu-core` types (this is where the deferred
   `#[derive(specta::Type)]` pass finally lands).

## Gotchas / notes for the next agent

- **The session-0 scaffolded CI files looked complete but would not have run** — no Linux
  system deps for Tauri is the kind of gap that only shows up on a real runner, not by reading
  the YAML. Always sanity-check a Tauri CI job's Linux deps against
  <https://v2.tauri.app/start/prerequisites/#linux> when touching `.github/actions/setup`.
- **`taiki-e/install-action` tool names are a guess for `sqlx-cli`/`typos-cli`** (not used in the
  final config precisely because of that uncertainty) — if a later agent wants to speed those
  up, confirm the exact tool key in that action's README/registry before relying on it, since an
  unrecognized name fails the whole composite step for every OS.
- **`gitleaks` and `lychee` are still not installed anywhere** (local or CI) — `just validate`
  has always treated them as optional, so this isn't a regression, but it does mean secret
  scanning and link checking are not actually happening yet. Pick this up before any milestone
  where that matters.
