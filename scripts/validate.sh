#!/usr/bin/env bash
# The validation gate. See docs/testing-and-validation.md for the full matrix.
#
# Design: run *every* step, collect failures, report them all at the end (not
# fail-fast — one run should surface every problem). Core checks always run and
# must pass. Optional external tools run when present and otherwise print a
# "(skip: <tool> not installed — `just setup`)" line; `just setup` and CI install
# them. A skip never fails the gate; a present tool that fails does.
set -uo pipefail

fail=0
skips=0
step() { echo; echo "==> $1"; }
run()  { "$@" || fail=1; }
have() { command -v "$1" >/dev/null 2>&1; }
skip() { echo "   (skip: $1)"; skips=$((skips + 1)); }

# --- always-on core checks --------------------------------------------------

step "progress files (scripts/progress-check.mjs)"
run node ./scripts/progress-check.mjs

step "emuprofile schema + fixtures"
run node ./scripts/validate-schema.mjs

step "emu-core stays tauri-free"
run bash ./scripts/emu-core-no-tauri.sh

# --- rust ----------------------------------------------------------------------

if [[ -f Cargo.toml ]]; then
  step "rust: fmt"
  run cargo fmt --all --check

  step "rust: clippy (-D warnings)"
  run cargo clippy --workspace --all-targets --all-features -- -D warnings

  step "rust: tests"
  if have cargo-nextest; then
    run cargo nextest run --workspace --all-features
  else
    skip "cargo-nextest — using plain cargo test"
    run cargo test --workspace --all-features
  fi

  step "rust: coverage threshold"
  if have cargo-llvm-cov; then
    run cargo llvm-cov --workspace --all-features --fail-under-lines 0
  else
    skip "cargo-llvm-cov (M0 coverage gate is n/a; threshold enforced from M1)"
  fi

  step "rust: dependency policy (cargo-deny)"
  if have cargo-deny; then run cargo deny check; else skip "cargo-deny not installed — \`just setup\`"; fi

  step "rust: unused deps (cargo-machete)"
  if have cargo-machete; then run cargo machete; else skip "cargo-machete not installed — \`just setup\`"; fi

  step "sqlx offline metadata current"
  if [[ -d .sqlx ]]; then
    run cargo sqlx prepare --check --workspace
  else
    skip "no .sqlx/ offline cache — no query! macros yet (task 0005; adopt in M3)"
  fi

  step "ipc bindings fresh"
  run just bindings
  run git diff --exit-code src/lib/bindings.ts
else
  echo; echo "(skip: no Cargo.toml)"
fi

# --- web ---------------------------------------------------------------------

if [[ -f package.json && -d node_modules ]]; then
  step "web: typecheck"
  run pnpm typecheck
  step "web: lint"
  run pnpm lint
  step "web: format check"
  run pnpm format:check
  step "web: tests"
  run pnpm test
  step "web: unused files/exports (knip)"
  if [[ -x node_modules/.bin/knip ]]; then run node_modules/.bin/knip; else skip "knip not installed"; fi
elif [[ -f package.json ]]; then
  echo; echo "(skip: package.json present but node_modules missing — run \`pnpm install\`)"
else
  echo; echo "(skip: no package.json)"
fi

# --- docs / workflows / secrets --------------------------------------------

step "markdown lint"
if [[ -x node_modules/.bin/markdownlint-cli2 ]]; then
  run node_modules/.bin/markdownlint-cli2
elif have markdownlint-cli2; then
  run markdownlint-cli2
else
  skip "markdownlint-cli2 not installed"
fi

step "link check (offline)"
if have lychee; then run lychee --offline --no-progress .; else skip "lychee not installed — \`just setup\`"; fi

step "typos"
if have typos; then run typos; else skip "typos not installed — \`just setup\`"; fi

step "GitHub Actions lint"
if have actionlint; then run actionlint; else skip "actionlint not installed — \`just setup\`"; fi

step "secret scan"
if have gitleaks; then run gitleaks detect --no-banner --redact; else skip "gitleaks not installed — \`just setup\`"; fi

# --- summary ---------------------------------------------------------------

echo
echo "-----------------------------------------------------------------------"
if [[ $fail -ne 0 ]]; then
  echo "validate: FAILED"
  exit 1
fi
if [[ $skips -ne 0 ]]; then
  echo "validate: OK ($skips optional check(s) skipped — see lines above)"
else
  echo "validate: OK (all checks ran)"
fi
