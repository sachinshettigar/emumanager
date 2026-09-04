# EmuManager task runner — the source of truth for commands (AGENTS.md §4).
# `package.json` scripts and the Makefile mirror the important ones.
# Recipes marked (M0) are stubs until their milestone-0 task lands.

set shell := ["bash", "-uc"]

_default:
    @just --list

# --- setup / run -------------------------------------------------------------

# Install toolchains + dev deps + git hooks. Idempotent.
setup:
    ./scripts/setup.sh

# Run the app in dev mode.
dev:
    pnpm tauri dev

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

# Regenerate src/lib/bindings.ts from Rust command signatures.
bindings:
    cargo run -p app --bin export-bindings 2>/dev/null || cargo test -p app export_bindings

# Create a new sqlx migration.
db-migrate name:
    cargo sqlx migrate add -r "{{name}}"

# Refresh .sqlx/ offline query metadata after changing a query.
db-prepare:
    cargo sqlx prepare --workspace

# --- tests -----------------------------------------------------------------------

test: test-rust test-web

test-rust:
    cargo nextest run --workspace

test-web:
    pnpm test

# Real sdkmanager/avdmanager against a scratch SDK dir. Not in `validate`.
test-integration:
    cargo test --workspace -- --ignored

# End-to-end: Playwright UI everywhere; tauri-driver on Linux/Windows.
e2e:
    pnpm e2e

# --- schema --------------------------------------------------------------------

schema-check:
    node ./scripts/validate-schema.mjs
