# 2026-09-04 — session 0 (Claude Code) — scaffold

## Worked on

- Milestone: M0 (setting it up, no code)
- Established scope, stack, and the harness-agnostic project structure.

## Changed

- **Scope:** Android only — ADR `docs/adr/0003`. iOS dropped (impossible off macOS).
- **Stack:** Tauri v2 + Rust workspace + React/TS — ADR `docs/adr/0002`.
- **Sharing:** recipe-only `.emuprofile` — ADR `docs/adr/0004`.
- **Context model:** everything in committed files, per-tool config is a stub — ADR
  `docs/adr/0005`.
- Wrote `AGENTS.md` (operating manual), `README.md`, per-tool stubs (`CLAUDE.md`,
  `.cursor/rules/00-agents.mdc`, `GEMINI.md`, `.github/copilot-instructions.md`, `.windsurfrules`,
  `.idx/airules.md`).
- Wrote `docs/spec.md`, `docs/architecture.md`, `docs/adr/0000`–`0005`,
  `docs/context/{glossary,domain-model}.md`.
- Wrote `MILESTONES.md` (M0–M7 with DoD + coverage gates) and `PROGRESS.md`.
- Set up `.agent/`: `README.md`, `state.json` + `state.schema.json`, `templates/`, tasks
  `0001`–`0009` (all M0, all `todo`).
- Wrote `docs/playbooks/` (7) and `docs/prompts/` (5).
- Added tooling config: `justfile`, `lefthook.yml`, `deny.toml`, `_typos.toml`,
  `.markdownlint-cli2.yaml`, `.editorconfig`, `.gitignore`, `docs/testing-and-validation.md`.
- Added `schemas/emuprofile/v1.schema.json` + valid/invalid fixtures.
- Added CI workflow stubs under `.github/workflows/`.
- Moved design wireframes to `docs/design/`.

## State now

- `just validate`: **not runnable yet** — no `justfile` implementation, no code. That is task
  `0007` (config files exist; the recipe body + scripts do not).
- `just progress`: not runnable yet (script is task `0006`); `.agent/state.json` is hand-consistent.
- `lastValidatedCommit`: n/a.

## Next action

Start task `0001` (Cargo workspace skeleton). Then `0003` (frontend shell) can proceed in
parallel. `0006`/`0007` (justfile + validate gate) should land early so every later task has a
real gate to run. Suggested order: 0001 → 0006 → 0003 → 0002 → 0004 → 0005 → 0007 → 0008 → 0009.

## Gotchas / notes for the next agent

- The two regen gates (`just bindings`, `just db-prepare`) will bite if you edit commands/queries
  and forget — `lefthook` (task `0008`) automates staging them; until that lands, run them by hand.
- `emu-core` must never gain a `tauri` dep — there is a dedicated check script (task `0006`) and
  CI job (task `0009`). Keep Tauri types in `src-tauri`.
- Pin exact versions when adding `sdkmanager`/`cmdline-tools` download URLs later (M1); Google
  rotates the "latest" filenames.
- Tauri WebDriver e2e has no macOS support — plan macOS verification as Playwright-against-dev +
  a manual checklist (already noted in `MILESTONES.md` M6).
