# Playbook: onboard a new AI tool / harness

Use when you start developing this repo in a tool that hasn't been used here before (Cursor,
Gemini CLI, Antigravity/Windsurf, Copilot, Aider, Zed, …), or when your harness quota ran out and
you're switching.

## 1. Point the tool at `AGENTS.md`

Only two stubs are committed today — `CLAUDE.md` and `GEMINI.md` — and each is a two-line pointer
to `AGENTS.md` with no copied rules. If your tool isn't one of those, create the file it reads
(from the table below) as the **same one-line pointer** and commit it. Never paste rules into a
stub — that's the whole point (ADR 0005).

> Pointer content, verbatim: `Read AGENTS.md at the repo root and the docs it links before editing. Do not duplicate its content here.`

| Tool | File it reads | Stub exists? |
| --- | --- | --- |
| Claude Code | `CLAUDE.md` | yes |
| Gemini CLI / Code Assist | `GEMINI.md` | yes |
| Cursor | `.cursor/rules/agents.mdc` (front-matter `alwaysApply: true`) | add when needed |
| GitHub Copilot | `.github/copilot-instructions.md` | add when needed |
| Windsurf / Antigravity | `.windsurfrules` | add when needed |
| Aider | `CONVENTIONS.md` | add when needed |
| Anything else | tell it: "Read `AGENTS.md` and the docs it links before editing." | n/a |

## 2. Load context (do this every fresh start)

1. Read `AGENTS.md` top to bottom.
2. Read `docs/spec.md` and `docs/architecture.md`.
3. Read the **newest** file in `.agent/journal/`.
4. Run `just progress` and open `.agent/state.json`.
5. `git log --oneline -15` for recent movement.

## 3. Verify the environment

```
just setup      # idempotent; installs toolchains + dev deps + git hooks
just validate   # should be green on main; if not, the journal/PROGRESS says why
```

If `just` isn't installed: `cargo install just` (or `brew`/`scoop`/`apt`). `package.json` scripts
mirror the key recipes if you truly can't get `just`.

## 4. Start working

Pick the next `todo` task for `currentMilestone`, follow `AGENTS.md` §3, finish with
`update-progress.md`.

## 5. If the tool has its own "memory" feature

Use it only for scratch. Anything durable goes in `docs/context/`, `docs/adr/`, `PROGRESS.md`, or
a journal entry — otherwise the next tool loses it.
