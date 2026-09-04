# ADR 0002: Tauri v2 + Rust core + React/TS UI

- Status: accepted
- Date: 2026-09-04
- Deciders: project owner

## Context

We need a single cross-platform (Windows/Linux/macOS) desktop app that spawns and supervises
child processes (`sdkmanager`, `emulator`, `adb`), does large verified downloads, inspects host
virtualization, and ships a small signed binary. It will be developed by AI agents across several
harnesses, so the codebase should be conventional and machine-checkable end to end.

## Decision

- **Tauri v2** as the shell.
- **Rust** for all backend logic, as a Cargo workspace: `src-tauri/` (Tauri glue only) plus
  `crates/emu-core` (domain, no `tauri` dep), `crates/emu-android`, `crates/emu-host`,
  `crates/emu-helper`.
- **React 18 + TypeScript (strict) + Vite** for the UI.
- **`tauri-specta`** to generate the TS IPC bindings from Rust, making the language seam
  type-checked on both sides.
- **`sqlx` + SQLite** for the registry, with committed offline query metadata.

## Consequences

- Easier: tiny signed binary; strong OS integration; the process/download/FS work is squarely in
  Rust's wheelhouse; the IPC boundary can't silently drift; `cargo`/`clippy`/`tsc` give agents a
  tight feedback loop.
- Harder: two languages; Rust async + lifetimes slow AI-assisted iteration vs. plain TS; Tauri
  WebDriver e2e is Linux/Windows only (no macOS) — macOS e2e is Playwright-against-dev-server plus
  manual release checks.
- We maintain two "regenerate then commit" gates (`just bindings`, `just db-prepare`); `lefthook`
  automates them on pre-commit to reduce agent thrash.

## Alternatives considered

- **Electron + TypeScript everywhere** — best AI-tool ergonomics and one language, but ~120 MB
  runtime, heavier memory, and a weaker security posture for a tool that spawns processes and runs
  an elevated helper. Rejected for footprint/security; revisit only if Rust velocity proves
  unworkable.
- **Wails v2 (Go)** — small binary, simple language, good concurrency, but a thinner desktop
  ecosystem and fewer examples for agents to lean on.
- **Rust + native GUI (egui/Slint)** — no webview, but far more UI code and worse fit for the
  wireframed layouts.
