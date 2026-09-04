# Playbooks

Step-by-step recipes for recurring work. These are the harness-agnostic equivalent of "skills":
any tool can follow them. Claude Code skill wrappers under `.claude/skills/` (if present) are 3
lines that point back here — edit the playbook, not the wrapper.

| Playbook | Use when |
| --- | --- |
| `update-progress.md` | You finished (or paused) a task — how to update state/PROGRESS/journal |
| `bootstrap-new-harness.md` | Onboarding a new AI tool (Cursor, Gemini, Antigravity, …) to this repo |
| `add-tauri-command.md` | Adding or changing an IPC command (includes the `just bindings` dance) |
| `add-migration.md` | Changing the DB schema / a `sqlx` query |
| `write-tests.md` | What to test and where, per layer |
| `debug-emulator-boot.md` | An emulator won't create or reach boot-complete |
| `milestone-review.md` | Closing out a milestone against its Definition of Done |
