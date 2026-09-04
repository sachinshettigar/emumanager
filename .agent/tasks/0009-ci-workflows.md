---
id: "0009"
title: "CI: ci.yml matrix + schema.yml + emu-core-no-tauri check"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
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

- [ ] `ci.yml`: matrix `{ubuntu-latest, windows-latest, macos-latest}` → composite setup →
      `just validate` → `tauri build --debug` (no signing) as an artifact
- [ ] Rust + pnpm caches keyed on lockfiles; cold run < ~15 min, warm < ~8 min
- [ ] A dedicated fast job runs `scripts/emu-core-no-tauri.sh` and `just progress` on ubuntu only
- [ ] `schema.yml`: on changes to `schemas/**` runs `node scripts/validate-schema.mjs`
- [ ] `nightly-integration.yml`: cron; sets up KVM on the ubuntu runner; runs
      `just test-integration` and `just e2e` (Linux + Windows); failures open/annotate an issue,
      don't block anything
- [ ] `actionlint` passes on all workflows (also part of `just validate`)
- [ ] Branch protection note added to `docs/playbooks/milestone-review.md` (require `ci.yml`)

## Validate

```
actionlint && gh workflow list   # and observe the first green run on a PR
```

## Notes / findings
