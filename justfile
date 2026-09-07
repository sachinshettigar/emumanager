# EmuManager task runner — the source of truth for commands (AGENTS.md §4).
# `package.json` scripts and the Makefile mirror the important ones.
# Recipes marked (M0) are stubs until their milestone-0 task lands.
#
# Windows: run recipes through `just` on Git Bash or WSL (the shell is set to
# bash below). `scripts/*.sh` are POSIX + coreutils; on native pwsh call the
# underlying `cargo` / `pnpm` commands directly. CI uses Linux/macOS bash and
# `bash {0}` on the windows runner.

set shell := ["bash", "-uc"]

_default:
    @just --list

# --- setup / run -------------------------------------------------------------

# Install toolchains + dev deps + git hooks. Idempotent.
setup:
    ./scripts/setup.sh

# (Re)install the git hooks (lefthook). Folded into `just setup`.
hooks:
    pnpm exec lefthook install

# Run the app in dev mode.
dev:
    pnpm tauri dev

# Build an unsigned installer for the current OS/arch (ADR 0007). Output in
# src-tauri/target/release/bundle/. CI does the 3-OS matrix (release.yml).
package:
    pnpm tauri build

# --- inner loop ------------------------------------------------------------------

# Fast feedback: fmt-check, typecheck, clippy, changed-crate tests.
check-fast:
    ./scripts/check-fast.sh

# The gate. Everything CI runs. Must be green before a task is "done".
validate:
    ./scripts/validate.sh

# Consistency-check .agent/state.json vs MILESTONES.md and the tasks dir.
progress:
    node ./scripts/progress-check.mjs

# --- codegen (run after the matching change) ----------------------------------

# Regenerate src/lib/bindings.ts from Rust command signatures (tauri-specta).
# The `export_bindings` test in `emumanager` (src-tauri/src/lib.rs) writes the file;
# CI asserts it is up to date with `git diff --exit-code src/lib/bindings.ts`.
bindings:
    cargo test -p emumanager --lib export::export_bindings -- --exact
    @echo "bindings: wrote src/lib/bindings.ts"

# Create a new (timestamped, forward-only) migration in migrations/.
db-migrate name:
    cargo sqlx migrate add "{{name}}"

# Refresh .sqlx/ offline query metadata. Only needed once code uses the checked
# `query!` macros (M3); until then there are no queries to cache.
db-prepare:
    cargo sqlx prepare --workspace

# --- tests -----------------------------------------------------------------------

# Run all test suites (Rust + web).
test: test-rust test-web

# Rust tests (nextest if present, else cargo test).
test-rust:
    #!/usr/bin/env bash
    set -uo pipefail
    if command -v cargo-nextest >/dev/null 2>&1; then
      cargo nextest run --workspace
    else
      cargo test --workspace
    fi

# Frontend tests (Vitest). No-op until task 0003 adds package.json deps.
test-web:
    #!/usr/bin/env bash
    set -uo pipefail
    if [[ -f package.json && -d node_modules ]]; then
      pnpm test
    else
      echo "test-web: no node_modules yet (task 0003) — skipping"
    fi

# Real sdkmanager/avdmanager against a scratch SDK dir. Not in `validate`.
test-integration:
    cargo test --workspace -- --ignored

# End-to-end: Playwright UI everywhere; tauri-driver on Linux/Windows.
e2e:
    pnpm e2e

# --- schema --------------------------------------------------------------------

# Validate .emuprofile JSON Schema + fixtures (full check needs ajv; see script).
schema-check:
    node ./scripts/validate-schema.mjs
