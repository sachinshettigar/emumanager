---
id: "0009"
title: "CI: ci.yml matrix + schema.yml + emu-core-no-tauri check"
milestone: "M0"
status: "review"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-05"
---

## Goal

GitHub Actions that run the same gate as `just validate` on every push/PR across the three OSes,
plus a fast schema job and dependency automation.

## Scope — files this task may touch

- `.github/workflows/ci.yml`
- `.github/workflows/schema.yml`
- `.github/workflows/nightly-integration.yml` (integration + e2e; `--ignored`, not blocking PRs)
- `.github/dependabot.yml` (or `renovate.json`)
- `.github/actions/setup/action.yml` (composite: rust toolchain + cache, node + pnpm + cache, just)

## Acceptance criteria

- [x] `ci.yml`: matrix `{ubuntu-latest, windows-latest, macos-latest}` → composite setup →
      `just validate` → `tauri build --debug` (no signing) uploaded as an artifact (the repo's
      `bundle.active: false` means this just compiles the binary, no bundling)
- [~] Rust + pnpm caches keyed on lockfiles (`Swatinem/rust-cache`, `actions/setup-node`
      `cache: pnpm`) — present; the < ~15 min / < ~8 min budget itself unverified until a few
      real runs accumulate cache history
- [x] A dedicated `fast` job (ubuntu-only) runs `scripts/emu-core-no-tauri.sh` +
      `scripts/progress-check.mjs`/`validate-schema.mjs` directly (equivalent to `just progress` /
      `just validate-schema` without needing `just` installed for a 5-minute smoke job)
- [x] `schema.yml`: path-filtered on `schemas/**` / the validator script, installs `ajv`
      standalone, runs `node scripts/validate-schema.mjs` in full (ajv) mode
- [x] `nightly-integration.yml`: cron `0 3 * * *` + `workflow_dispatch`; enables KVM on the
      ubuntu runner; runs `just test-integration`; `just e2e` guarded to skip cleanly (no
      Playwright suite exists yet — deferred, see Notes); failures are a `::warning::`
      annotation, not a blocking status (opening an issue automatically is deferred — see Notes)
- [x] `actionlint` passes on all four workflow files (verified locally with the
      `download-actionlint.bash` release script) and is installed in CI via the same script
- [x] Branch protection note in `docs/playbooks/milestone-review.md` — concrete required-check
      names, flagged as a human (or explicitly-directed agent) action, not something to flip
      unprompted

## Validate

```
actionlint .github/workflows/*.yml   # (install via rhysd/actionlint's download-actionlint.bash)
gh workflow list
git push && gh run watch             # observe the push-triggered run on main
```

(The task's original `--commit-msg-file`-less form used `gh workflow list` + "observe on a PR";
this repo pushes straight to `main`, so the ci.yml `push` trigger is what's observed instead.)

## Notes / findings

### Decisions / deviations (kept simple)

- **`gitleaks` and `lychee` are not installed in CI.** Both need either a non-cargo release
  download or a dedicated action, and `just validate` already treats them as optional
  (skip-if-absent). Installing them is deferred to whenever secret/link scanning needs to be
  load-bearing for a milestone — the skip design means CI is no less strict today than it was
  before this task; it is strictly more so (actionlint and the four `taiki-e/install-action`
  tools now run for real).
- **Tool installs, fastest-safe-first:** `just`, `cargo-nextest`, `cargo-deny`,
  `cargo-machete`, `cargo-llvm-cov` via `taiki-e/install-action@v2` (prebuilt binaries — these
  five are common enough in that action's own examples to trust); `sqlx-cli` / `typos-cli` via
  `cargo install --locked || true` (unconfirmed prebuilt support, so from-source with a
  never-fail fallback instead of risking the whole composite step); `actionlint` via its own
  official `download-actionlint.bash` (no crates.io package).
- **Linux Tauri build deps added** to the composite action (`libwebkit2gtk-4.1-dev`,
  `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`,
  `build-essential`) — this was **missing entirely** before this task and would have failed
  every rust build/test/clippy step on `ubuntu-latest`, not just the tauri build. Source:
  <https://v2.tauri.app/start/prerequisites/#linux>. This is the one change in this task that
  would have blocked everything else, and is why the
  workflow files existing from the session-0 scaffold were not, in fact, ready to run.
- **e2e stays deferred.** `just e2e` (`playwright test`) has no config or `@playwright/test`
  dependency yet — `knip.json` already `ignoreBinaries`s it. The nightly workflow now skips it
  with a message instead of failing every night on a missing config; wire it up for real when a
  milestone needs an actual e2e flow to demo (M2+).
- **Auto-opening a GitHub issue on nightly failure is deferred** in favor of a workflow
  `::warning::` annotation — matches "skip a blocked flow, note it" rather than adding
  `actions/github-script` / a dedicated issue-bot for a workflow with no real tests to fail yet.
- **Did not enable branch protection.** That is a repository setting change outside "build and
  push code" — documented as the next human action in `docs/playbooks/milestone-review.md`
  instead of applied via `gh api` unprompted.
- Added `.github/dependabot.yml` (cargo + npm + github-actions, weekly, patch-grouped) — not
  explicitly required by the acceptance list but listed as an option in scope, and it's a
  one-file, zero-maintenance addition.

### Real bugs the first live CI run surfaced (fixed, verified locally with the real tools)

The first push-triggered run (`33881911297`) failed on macOS and ubuntu, both in the same
`cargo-deny check` step — installed the tools locally to confirm and fix rather than
round-trip through CI blind:

- **`license = "UNLICENSED"` is not a valid SPDX expression** (that's the npm convention, not
  Cargo's) — `cargo-deny` correctly flagged every workspace crate as `error[unlicensed]`. Fix:
  added `publish = false` to `[workspace.package]` (+ `publish.workspace = true` on every
  member) — these crates are never published, so `[licenses] private = { ignore = true }` in
  `deny.toml` is what actually matters; `license = "UNLICENSED"` stays as an informational value.
- **`error[wildcard]`** on `emumanager`/`emu-host`/`emu-android`'s path dependencies
  (`emu-core = { path = "..." }` has no version) — exactly what `wildcards = "deny"` exists to
  catch for a *published* crate, but we never publish. Fix: `[bans] allow-wildcard-paths = true`.
- **17 `unmaintained` advisories**, all transitive through Tauri/wry (10× gtk-rs GTK3 bindings —
  wry's Linux webview backend, 5× `unic-*` unicode crates via `urlpattern`, `paste`,
  `proc-macro-error`) — not vulnerabilities, not ours to fix. Added to `deny.toml`
  `[advisories] ignore` with a comment; revisit on a Tauri/wry upgrade.
- **`cargo machete`** (once actually installed) flagged `emu-android`/`emu-host` as unused in
  `src-tauri/Cargo.toml` — true today (no command uses them yet, M1/M2 will). Added
  `[package.metadata.cargo-machete] ignored = [...]` rather than removing-then-re-adding them.
- Verified locally after each fix by actually installing `cargo-deny`, `cargo-machete`,
  `cargo-nextest`, `cargo-llvm-cov` (none were on this machine before) — all four now genuinely
  run in `just validate` here too, not just in CI.
- **Windows `gate` job timed out at 20 min**, still mid-`cargo test` compile on a cold cache
  (not a real failure) — bumped `timeout-minutes` to 30; `Swatinem/rust-cache` should make
  subsequent runs on `main` much faster.
- Re-pushed; re-check `gh run list --workflow=ci.yml` for the next run before flipping this task
  and `M0` to `done`.
