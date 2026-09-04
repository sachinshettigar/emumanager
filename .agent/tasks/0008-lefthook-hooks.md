---
id: "0008"
title: "lefthook hooks (pre-commit regen+fmt, pre-push validate)"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-05"
---

## Goal

Git hooks that keep the two "regenerate or CI fails" gates (specta bindings, sqlx offline data)
from ever surprising an agent, and run cheap checks before commit / the full gate before push.

## Scope — files this task may touch

- `lefthook.yml`
- `scripts/hooks/` (any helper scripts the hooks call)
- `justfile` (a `hooks-install` step folded into `setup`)
- `docs/playbooks/` (note the hook behavior where relevant)

## Acceptance criteria

- [x] `pre-commit`: `cargo fmt` on staged `*.rs` + `prettier --write` on staged
      `*.{ts,tsx,js,jsx,json,css}` (+ re-`git add`); `just bindings` when `*.rs`/`*.toml` staged,
      re-`git add src/lib/bindings.ts`
- [x] `pre-commit` also runs `gitleaks protect --staged` (guarded: `command -v gitleaks`, prints
      a skip line otherwise) and `just progress`
- [x] `pre-push`: runs `just validate`; blocks on failure. Override `LEFTHOOK=0 git push`
      (documented in the `lefthook.yml` header); auto-skips when `CI` is set
- [x] `commit-msg`: Conventional Commits regex
      (`feat|fix|chore|docs|refactor|test|perf|build|ci`, optional scope, optional `!`)
- [x] `lefthook` is a dev-dependency (npm, bundles the binary); `just setup` runs
      `pnpm exec lefthook install`; `just hooks` re-installs. Fresh clone: `pnpm install` +
      `just setup` (or `just hooks`) gets them
- [x] CI skipped cleanly — CI never triggers git hooks, and `pre-push` also self-skips on `CI`

- [~] `just db-prepare` in `pre-commit` is a **guarded no-op** until M3: only runs when `.sqlx/`
      and `cargo-sqlx` both exist (no `query!` macros yet — see task 0005). Same guard pattern
      as `scripts/validate.sh`.

## Validate

```
pnpm exec lefthook run pre-commit
printf 'feat(core): x\n' > /tmp/m && pnpm exec lefthook run commit-msg /tmp/m   # exit 0
printf 'bad\n'          > /tmp/m && pnpm exec lefthook run commit-msg /tmp/m   # exit 1
```

(lefthook v2 dropped `--commit-msg-file`; the message file is a positional arg.)

## Notes / findings

### Decisions / deviations (kept simple)

- **`lefthook` via npm** (`2.1.12`, devDep) instead of a system binary — cross-platform, one
  `pnpm install`, no brew/scoop step. `pnpm` skips its postinstall build script but the
  platform binary still resolves via optionalDependencies (`pnpm exec lefthook version` works).
- **`db-prepare` + `secrets` hooks are guarded**, not removed — they light up when the tool /
  `.sqlx/` appears (M3 for sqlx, `just setup`/brew for gitleaks) without editing the config.
- **`bindings` hook is scoped to `*.{rs,toml}`** so doc-only commits don't pay a Rust compile.
- **`pre-push` respects `CI`** as well as never being triggered by CI, belt-and-braces.
- The hooks are **installed and active as of this commit** — this repo's own commits now go
  through them.
