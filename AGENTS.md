# AGENTS.md — Emulator Studio operating manual

**This file is the single source of truth for any AI coding agent or human working on this repo.**
Every harness (Claude Code, Cursor, Gemini CLI, Antigravity/Windsurf, Copilot, Aider, …) is
configured with a thin stub that redirects here. If you are an agent: read this file top to
bottom, then read the docs it points to, before editing code.

Nothing important lives in a tool's private memory. Context, decisions, and progress all live in
committed files so work survives a switch between tools.

---

## 1. What we are building

A cross-platform **desktop app to create, launch, track, and share Android emulators** without
installing Android Studio. Scope is deliberately **Android only** (see `docs/adr/0003`).

- Product spec: `docs/spec.md`
- Architecture: `docs/architecture.md`
- Milestones + definition of done: `MILESTONES.md`
- Current state: `PROGRESS.md` and `.agent/state.json`
- Design (wireframes): `docs/design/wireframes/` (open `docs/design/emumanager-wireframes.html`)
- Domain vocabulary: `docs/context/glossary.md`, `docs/context/domain-model.md`

## 2. Stack (fixed — changing it requires an ADR)

| Layer | Choice |
| --- | --- |
| Shell | Tauri v2 |
| Backend | Rust — workspace: `src-tauri/` (thin shell) + `crates/emu-core`, `crates/emu-android`, `crates/emu-host`, `crates/emu-helper` |
| Frontend | Vite + React 18 + TypeScript (strict), TanStack Query for command/event data, Zustand for local UI state |
| IPC | `tauri-specta` — Rust command signatures generate `src/lib/bindings.ts`. Never hand-write that file. |
| DB | `sqlx` + SQLite, `migrations/`, offline metadata in `.sqlx/` (committed) |
| Styling | Tailwind + tokens in `src/styles/tokens.css` (mirrors the wireframe system: IBM Plex Sans/Mono, blue primary, green = running/installed, amber = attention) |
| Task runner | `just` (primary) — mirrored by `package.json` scripts and `make` |
| Git hooks | `lefthook` |
| CI | GitHub Actions, matrix ubuntu/windows/macos |

Rationale: `docs/adr/0002-tauri-v2-stack.md`.

## 3. The development loop

1. **Pick a task.** Open `.agent/state.json` → `currentMilestone`. Find the next task with
   `status: "todo"` in `.agent/tasks/` for that milestone. Read the whole task file.
2. **Move it to `doing`** (edit the task file's front matter and `.agent/state.json`).
3. **Implement.** Stay inside the task's stated scope and file list. If you discover the task is
   wrong or too big, stop and update the task file with what you learned rather than sprawling.
4. **Validate fast, often:** `just check-fast` (fmt + typecheck + clippy + affected tests).
5. **Full gate before you call it done:** `just validate` must pass with zero warnings.
6. **Update progress** — this is not optional, it is how the next agent continues:
   - Tick the task's acceptance criteria; set `status: "review"` (or `done` if trivially verifiable).
   - Update `PROGRESS.md` (the narrative + milestone checkboxes).
   - Update `.agent/state.json` (`tasks[].status`, `lastValidatedCommit`).
   - Append a journal entry: copy `.agent/templates/journal.md` to
     `.agent/journal/<YYYY-MM-DD>-<slug>.md` and fill it in.
7. **Commit** using Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`,
   `test:`). One logical change per commit. Reference the task id: `feat(android): create AVD (#0007)`.

Follow `docs/playbooks/update-progress.md` for the exact edits in step 6.

## 4. Commands (the only ones you need)

| Command | Does |
| --- | --- |
| `just setup` | Install toolchains + dev dependencies (idempotent). Run once per machine. |
| `just dev` | Run the app in dev mode (`tauri dev`). |
| `just check-fast` | Inner loop: fmt-check, `tsc --noEmit`, `clippy -D warnings`, changed-crate tests. |
| `just validate` | **The gate.** Everything CI runs: all of check-fast + full test suites + coverage thresholds + `cargo deny` + schema fixtures + lint of docs/workflows + unused-code scan + secret scan. |
| `just bindings` | Regenerate `src/lib/bindings.ts` from Rust. Run after changing any `#[tauri::command]`. |
| `just db-migrate "<name>"` | Create a new SQLx migration. |
| `just db-prepare` | Regenerate `.sqlx/` offline data after changing a query. |
| `just test` / `just test-rust` / `just test-web` | Test suites. |
| `just e2e` | End-to-end (Playwright UI; `tauri-driver` on Linux/Windows). |
| `just progress` | Consistency-check `.agent/state.json` against `MILESTONES.md` and the tasks dir. |

`lefthook` runs `bindings` + `db-prepare` + fmt on pre-commit and stages the results, so the two
"regenerate or CI fails" gates are handled for you if you commit through git normally.

## 5. Repo map

```
AGENTS.md                 you are here
README.md                 humans: build & run
MILESTONES.md             M0..M7, each with a Definition of Done
PROGRESS.md               current narrative state + checkboxes
justfile                  task runner (source of truth for commands)
lefthook.yml              git hooks -> scripts/
deny.toml _typos.toml .markdownlint-cli2.yaml .editorconfig   linter configs

docs/
  spec.md                 product spec (scope B: Android only)
  architecture.md         components, data flow, boundaries
  adr/                    architecture decision records (append-only)
  context/                glossary, domain model — durable knowledge
  playbooks/              step-by-step recipes ("skills") — harness-agnostic
  prompts/                reusable prompt templates you paste into any tool
  design/                 wireframes + design notes

.agent/                   tool-neutral working state (replaces proprietary "memory")
  state.json              machine-readable progress
  tasks/NNNN-slug.md      one file per unit of work; has acceptance criteria + validate cmd
  journal/                dated session handoff notes (append-only)
  templates/              task.md, journal.md

schemas/emuprofile/       .emuprofile JSON Schema + valid/invalid fixtures

src/                      frontend (React/TS)
src-tauri/                Tauri shell: commands, events, wiring only — no domain logic
crates/
  emu-core/               domain: EmulatorProvider trait, models, profile engine, registry, jobs. NO tauri dependency.
  emu-android/            the only provider impl: wraps sdkmanager/avdmanager/emulator/adb
  emu-host/               host + acceleration inspection
  emu-helper/             tiny separate elevated binary (enable WHPX, add kvm group, ...)
migrations/               sqlx migrations
.sqlx/                    sqlx offline query metadata (committed)
```

## 6. Golden rules

1. **`crates/emu-core` never depends on `tauri`.** Domain logic must be testable with `cargo test`
   alone. `src-tauri` is glue.
2. **Never invent Android SDK behavior.** `sdkmanager`, `avdmanager`, `emulator`, `adb` flags and
   output formats are load-bearing. Cite the source (official docs or `--help` output captured in a
   test fixture) in a comment. Parsing is done against captured fixtures in `crates/emu-android/tests/fixtures/`.
3. **The Rust↔TS seam is typed.** All IPC goes through `tauri-specta` bindings. If you add a
   command, run `just bindings` and commit the diff.
4. **No network in unit tests.** Toolchain downloads, `adb`, and process spawning are behind traits
   with fake impls for tests. Real-binary tests live behind `#[ignore]` / a `just test-integration`
   target and are not part of `just validate`.
5. **No secrets, no personal data in the repo.** Signing keys, Apple/Google credentials, tokens →
   CI secrets only. `gitleaks` runs in `just validate`.
6. **Scope discipline.** A task lists the files it touches. Touch those. New cross-cutting concern →
   new task or an ADR, not scope creep.
7. **Every user-facing string is real and specific.** No lorem, no "An error occurred". Errors say
   what happened and what to do.
8. **Keep `just validate` green on every commit to `main`.** If you must land something red, the
   task file and PROGRESS.md say so explicitly with the reason.

## 7. Coding standards

- **Rust:** `rustfmt` default. `clippy` with `-D warnings`; the `pedantic` group is on with targeted
  `allow`s in `clippy.toml`. `thiserror` for library errors, `anyhow` only in `src-tauri`/binaries.
  `tracing` for logs, never `println!`. Public items in `emu-core` get doc comments.
- **TypeScript:** `strict: true`, no `any` (use `unknown` + narrowing). `typescript-eslint`
  strict-type-checked. Components are function components; data fetching via TanStack Query wrapping
  the specta bindings; no `fetch` to localhost. One component per file.
- **Tests:** Rust unit tests next to code; crate-level integration tests in `tests/`. Frontend:
  Vitest + Testing Library, colocated `*.test.tsx`. Coverage gate in `just validate` (start at 60%,
  ratchet up per milestone — see `MILESTONES.md`).
- **Commits:** Conventional Commits. **PRs/commits never claim a test passed that you did not run.**

## 8. Switching harnesses / onboarding a new tool

There are only two per-tool stub files today — `CLAUDE.md` and `GEMINI.md` — and each is a
two-line pointer to this file with **no copied content**. When you start using a new tool
(Cursor, Antigravity, Copilot, Aider, …), add its native config file as the same kind of
one-line pointer ("Read `AGENTS.md` and the docs it links before editing.") and commit it — never
paste rules into it. `docs/playbooks/bootstrap-new-harness.md` lists which filename each tool
reads.

To hand off mid-stream, follow `docs/prompts/handoff.md` and leave a journal entry. A new agent
should be able to run `just progress` + read the latest journal entry + read `PROGRESS.md` and
know exactly where to resume.

## 9. When you are unsure

- Behavior of an SDK tool → capture `--help` / real output into a fixture, write the parser against
  it, cite it.
- A design decision with trade-offs → write an ADR (`docs/adr/`), don't decide silently in code.
- The task seems wrong → edit the task file, set `status: "blocked"` with a note, pick another.
- Product intent → `docs/spec.md`; if it's silent, add a `> QUESTION:` line to the spec and flag it
  in your journal entry rather than guessing.
