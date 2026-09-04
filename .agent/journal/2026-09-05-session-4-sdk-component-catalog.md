# 2026-09-05 — session 4 (Claude Code)

## Worked on

- Task(s): `0010` (done), `0011`/`0012`/`0013` (filed as `todo`)
- Milestone: `M1`

## Changed

- `crates/emu-core/src/model/component.rs` (new) — `Component`, `ComponentId`, `HostOs`,
  `HostArch`; re-exported from `crates/emu-core/src/model/mod.rs`.
- `crates/emu-android/src/catalog.rs` (new) — `parse(xml, os, arch)` over Google's real
  repository manifest; `roxmltree` dep added to `crates/emu-android/Cargo.toml`.
- `crates/emu-android/tests/fixtures/repository2-3.xml` (new) — real, trimmed capture (see the
  task file's Notes for exact `curl` command + quirks found).
- `.agent/tasks/0010-sdk-component-catalog.md` → `done`; `.agent/tasks/0011-native-ports.md`,
  `0012-toolchain-bootstrap.md`, `0013-dependencies-screen.md` filed as `todo` (M1's remaining
  scope, broken down the same granularity as M0's tasks).
- `.agent/state.json`, `MILESTONES.md`, `PROGRESS.md` updated (see PROGRESS.md's log entry for
  the full narrative, including the CI billing-block finding).
- Installed `just` via `brew` — it was missing at the start of this session even though the
  justfile/package.json assume it; `cargo-deny`/`cargo-machete`/`cargo-nextest`/`cargo-llvm-cov`
  from a prior session were still present.

## State now

- `just validate`: **pass** (5 optional checks skip locally: lychee, typos, actionlint,
  gitleaks — same as every prior session; nothing new)
- `just progress`: pass (M1, 13 task(s), 13 task file(s))
- Tasks moved: `0010` doing→done; `0011`/`0012`/`0013` created as `todo`
- `lastValidatedCommit` in state.json: set to this session's commit (see git log) after pushing
- M0's `0009` stays `review` — not a code issue, see Gotchas

## Next action

Pick up task `0011` (native `ProcessRunner`/`Downloader`/`Clock`/`Fs` port impls in `src-tauri`).
Two decisions are flagged in its Notes placeholder and must be recorded, not silently resolved:
how SHA-1 (from the repo manifest) reconciles with `Downloader::fetch`'s SHA-256 verification,
and where the real HTTP/process code should live (task file argues for `src-tauri`, not
`emu-android`, per the architecture doc's crate-boundary table — re-check that reasoning still
holds before writing code).

## Gotchas / notes for the next agent

- **CI is blocked on GitHub billing, not code.** The push-triggered run after the `0009` bugfix
  commit (`a97606e`) never started: `gh run view` shows "recent account payments have failed or
  your spending limit needs to be increased." This needs a human in GitHub
  **Settings → Billing & plans** — do not spend more time debugging workflow YAML for this.
  Once fixed, re-push (or re-run) and watch `ci.yml`; if green, flip `0009` to `done`, `M0` to
  `done` in `.agent/state.json`, and set `currentMilestone` to confirm `M1` (already set).
- **`just` was not installed** on this machine at the start of this session (despite being used
  successfully in earlier sessions per PROGRESS.md) — installed via `brew install just`. If a
  future session hits "command not found: just" again, that's the fix, not a project bug.
- **`dist/index.html` is a tracked file that shouldn't be** (committed accidentally in task
  0001, before build outputs were gitignored). It picks up spurious diffs from any local
  `pnpm build`/`tauri build`. This session discarded such a diff with `git checkout -- dist/
  index.html` rather than committing it or adding it to scope — worth a small standalone
  cleanup task (`git rm --cached dist/index.html` + gitignore it) whenever someone's touching
  build config, but not done here to stay in scope.
- **Repo XML gives SHA-1; `ports::Downloader::fetch` verifies SHA-256.** Not a bug in either —
  just a mismatch to resolve deliberately in task `0011`/`0012` (options: verify SHA-1 before
  calling `fetch` with `expected_sha256: None`, or widen the trait to a `Checksum` enum). Don't
  let a download silently skip verification to make the mismatch go away.
- **Spec says "bundled JRE"; modern `cmdline-tools` needs a system JDK 17+.** Google stopped
  shipping a JRE with cmdline-tools years ago. Task `0012` must record a real decision (recommend:
  v1 requires a system JDK with a clear, actionable error; defer real JRE bundling to a follow-up
  if user testing shows it's needed) instead of discovering this mid-implementation.
- System images (`system-images;...`) are **not** part of the M1 catalog — different Google feed,
  M2 scope, already has `ImageCoord`/`SystemImage` from task 0002. Don't accidentally fold them
  into `0011`/`0012`.
