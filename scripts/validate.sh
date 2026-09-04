#!/usr/bin/env bash
# The validation gate. See docs/testing-and-validation.md for the full matrix.
# Steps light up as their M0 tasks land; today it runs what already works.
set -uo pipefail

fail=0
step() { echo; echo "==> $1"; }
run()  { "$@" || fail=1; }
have() { command -v "$1" >/dev/null 2>&1; }

step "progress files (scripts/progress-check.mjs)"
run node ./scripts/progress-check.mjs

step "emuprofile schema + fixtures"
run node ./scripts/validate-schema.mjs

step "emu-core stays tauri-free"
run bash ./scripts/emu-core-no-tauri.sh

if [[ -f Cargo.toml ]]; then
  step "rust: fmt / clippy / tests / deny";        run cargo fmt --all --check
  run cargo clippy --workspace --all-targets -- -D warnings
  if have cargo-nextest; then run cargo nextest run --workspace; else run cargo test --workspace; fi
  if have cargo-deny; then run cargo deny check; else echo "   (skip: cargo-deny not installed)"; fi
  if have cargo-machete; then run cargo machete; else echo "   (skip: cargo-machete not installed)"; fi
  if [[ -d .sqlx ]]; then
    step "sqlx offline metadata current"
    run cargo sqlx prepare --check --workspace
  else
    echo; echo "   (skip: no .sqlx/ offline cache — no query! macros yet, see task 0005)"
  fi
  step "ipc bindings fresh"
  run just bindings && run git diff --exit-code src/lib/bindings.ts
else
  echo; echo "(skip: no Cargo.toml yet — pre task 0001)"
fi

if [[ -f package.json && -d node_modules ]]; then
  step "web: typecheck / lint / format / test / knip"
  run pnpm typecheck && run pnpm lint && run pnpm format:check && run pnpm test
  if have pnpm && pnpm exec knip --version >/dev/null 2>&1; then run pnpm knip; fi
elif [[ -f package.json ]]; then
  echo; echo "(skip: package.json present but node_modules missing — run \`pnpm install\` / task 0003)"
else
  echo; echo "(skip: no package.json yet — pre task 0003)"
fi

step "docs / workflows / secrets"
have markdownlint-cli2 && run markdownlint-cli2 || echo "   (skip: markdownlint-cli2)"
have lychee            && run lychee --offline --no-progress . || echo "   (skip: lychee)"
have typos             && run typos || echo "   (skip: typos)"
have actionlint        && run actionlint || echo "   (skip: actionlint)"
have gitleaks          && run gitleaks detect --no-banner --redact || echo "   (skip: gitleaks)"

echo
if [[ $fail -ne 0 ]]; then echo "validate: FAILED"; exit 1; fi
echo "validate: OK"
