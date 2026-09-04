# ADR 0001: Record architecture decisions

- Status: accepted
- Date: 2026-09-04
- Deciders: project owner

## Context

The project will be built largely by AI coding agents, across multiple tools, over a long period.
Decisions made silently in code are invisible to the next agent and get silently reversed.

## Decision

Every non-trivial architectural or technology decision is captured as a numbered ADR in
`docs/adr/`, using `0000-template.md`. ADRs are append-only: to change a decision, add a new ADR
that supersedes the old one and update the old one's status line.

An agent that finds itself choosing between options with real trade-offs must stop and write an
ADR rather than deciding in a commit.

## Consequences

- Easier: onboarding a new tool/agent; understanding why the code is shaped as it is.
- Harder: slightly more ceremony per decision.
- We maintain a small, high-signal decision log instead of tribal knowledge.

## Alternatives considered

- **Decisions in commit messages** — not discoverable, no lifecycle.
- **A single DECISIONS.md** — merges poorly, hard to reference a specific decision.
