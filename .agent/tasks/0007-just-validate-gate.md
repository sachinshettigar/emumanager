---
id: "0007"
title: "just validate wired end to end (all static tools)"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
---

## Goal

`just validate` runs the full static-analysis + test gate described in
`docs/testing-and-validation.md` and passes on a clean checkout.

## Scope — files this task may touch

- `scripts/validate.sh`
- Tool configs: `deny.toml`, `_typos.toml`, `.markdownlint-cli2.yaml`, `lychee.toml`,
  `.gitleaks.toml`, `knip.json`, `rustfmt.toml`, `clippy.toml`
- `justfile` (`validate` recipe body)
- `.config/nextest.toml`

## Acceptance criteria

- [ ] `just validate` runs, in order, failing fast:
  - `cargo fmt --check` · `cargo clippy --workspace -- -D warnings`
  - `scripts/emu-core-no-tauri.sh`
  - `cargo nextest run --workspace` · coverage via `cargo llvm-cov` with the M-gate threshold
  - `cargo sqlx prepare --check --workspace`
  - `cargo deny check` · `cargo machete`
  - `just bindings && git diff --exit-code src/lib/bindings.ts`
  - `pnpm typecheck` · `pnpm lint` · `pnpm format:check` · `pnpm test` (with coverage threshold) · `pnpm knip`
  - schema: `node scripts/validate-schema.mjs` (ajv: schema self-valid + all fixtures)
  - `markdownlint-cli2` · `lychee --offline .` · `typos` · `actionlint` · `gitleaks detect`
  - `just progress`
- [ ] Every tool above is either a dev-dependency or installed by `just setup`; `validate.sh`
      prints a clear message if one is missing
- [ ] Runs green on macOS locally and on the CI matrix (task 0009)
- [ ] Total runtime on CI < ~8 min (parallelize rust/web where sensible)
- [ ] `docs/testing-and-validation.md` written: the matrix, what each tool catches, how to run one

## Validate

```
just validate
```

## Notes / findings
