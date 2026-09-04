# ADR 0005: Harness-agnostic project context

- Status: accepted
- Date: 2026-09-04
- Deciders: project owner

## Context

Development must be able to continue in a different AI tool at any time (Claude Code → Cursor →
Gemini CLI → Antigravity/Windsurf → Copilot → Aider). Anything stored in one tool's private
memory, rules format, or skill format is lost on a switch.

## Decision

All context, decisions, playbooks, and progress live in **plain committed files** at
conventional paths:

- `AGENTS.md` — the one operating manual. Each tool's native config file is a **two-line pointer
  with no copied content** ("Read `AGENTS.md` …"). Only the stubs for tools actually in use are
  committed — today `CLAUDE.md` and `GEMINI.md`. Adding a tool means adding one more one-line
  pointer, never pasting rules.
- `docs/` — spec, architecture, ADRs, durable domain context, playbooks, prompts.
- `.agent/` — tool-neutral working state: `state.json` (machine-readable progress), `tasks/`
  (units of work with acceptance criteria), `journal/` (dated handoff notes), `templates/`.
- Validation is a set of **plain scripts** invoked by one command (`just validate`) that CI, git
  hooks, and any agent all call identically. No tool-specific automation holds logic.
- "Skills" are `docs/playbooks/*.md`. Claude Code skill wrappers (if any) are 3 lines pointing at
  the playbook.
- "Memory" is `docs/context/` + `docs/adr/` + `PROGRESS.md` + `.agent/`.

## Consequences

- Easier: any tool can be productive after reading `AGENTS.md`; handoffs are a journal entry;
  nothing is trapped.
- Harder: agents must be disciplined about updating the files (enforced by `just progress`, the
  handoff prompt, and PR review).
- Some duplication in the stub files; kept to a few lines each.

## Alternatives considered

- **Lean on each tool's native memory/rules** — fastest per tool, but the switch cost is total
  context loss. Rejected — portability is a hard requirement.
- **A custom MCP "project context" server** — powerful, but it's itself a dependency not every
  harness supports, and it hides state that should be in git. Rejected for v1.
