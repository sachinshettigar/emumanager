---
id: "0008"
title: "lefthook hooks (pre-commit regen+fmt, pre-push validate)"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
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

- [ ] `pre-commit`: runs `cargo fmt` + `prettier --write` on staged files, `just bindings`,
      `just db-prepare`, then `git add` of any regenerated `src/lib/bindings.ts` / `.sqlx/`
- [ ] `pre-commit` also runs `gitleaks protect --staged` and `just progress`
- [ ] `pre-push`: runs `just validate`; blocks the push on failure (overridable with
      `LEFTHOOK=0` for explicit WIP branches, documented)
- [ ] `commit-msg`: conventional-commit lint (lightweight regex or `commitlint`)
- [ ] `just setup` installs the hooks (`lefthook install`); a fresh clone that runs setup gets them
- [ ] Hooks are skipped cleanly in CI (CI runs `just validate` directly)

## Validate

```
lefthook run pre-commit && lefthook run commit-msg --commit-msg-file <(echo "feat(core): x")
```

## Notes / findings
