# Progress

Narrative companion to `.agent/state.json`. Update both together (see
`docs/playbooks/update-progress.md`). Newest entries at the top of the log.

## Current state

- **Milestone:** M0 — Skeleton & gate
- **Phase:** scaffolding only. No application code exists yet — repo currently holds docs, the
  AI-development harness (`AGENTS.md`, `.agent/`, playbooks, prompts), planning
  (`MILESTONES.md`), tooling config, the `.emuprofile` schema, and the design wireframes.
- **Last validated commit:** _none — `just validate` not yet runnable (M0 task 0007)_
- **Next action:** task `0001` (Cargo workspace skeleton) in `.agent/tasks/`.

## Milestone checklist

- [~] **M0** Skeleton & gate — scaffolding done; code tasks 0001–0009 open
- [ ] M1 Toolchain manager: SDK from zero
- [ ] M2 Create & launch one emulator end-to-end
- [ ] M3 Registry & reliable tracking
- [ ] M4 Profiles: export / import / recreate
- [ ] M5 Host readiness & elevated helper
- [ ] M6 Cross-platform hardening & packaging
- [ ] M7 Feature-complete v1.0

## Log

### 2026-09-04 — session 0 (Claude Code) — scaffold

- Set product scope to **B: Android only** (ADR 0003) and stack to **Tauri v2 + Rust + React/TS**
  (ADR 0002).
- Created the harness-agnostic project structure (ADR 0005): `AGENTS.md` + per-tool stubs,
  `docs/` (spec, architecture, ADRs 0001–0005, glossary, domain model), `MILESTONES.md`,
  `.agent/` working state, `docs/playbooks/` + `docs/prompts/`, tooling config
  (`justfile`, `lefthook.yml`, linters), `schemas/emuprofile/v1.schema.json` + fixtures, CI
  workflows.
- Design wireframes moved to `docs/design/`.
- No code yet. M0 code tasks (`0001`–`0009`) are written and `todo`.
- Next agent: start at task `0001`; run `just progress` to sanity-check state.
