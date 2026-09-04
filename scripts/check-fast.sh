#!/usr/bin/env bash
# Inner-loop check: fast subset of the gate. Full gate is scripts/validate.sh.
set -uo pipefail
fail=0
run() { "$@" || fail=1; }
have() { command -v "$1" >/dev/null 2>&1; }

run node ./scripts/progress-check.mjs
run bash ./scripts/emu-core-no-tauri.sh

if [[ -f Cargo.toml ]]; then
  run cargo fmt --all --check
  run cargo clippy --workspace -- -D warnings
  # affected tests only where possible; fall back to core
  run cargo test -p emu-core
fi

if [[ -f package.json && -d node_modules ]]; then
  run pnpm typecheck
  run pnpm lint
elif [[ -f package.json ]]; then
  echo "check-fast: (skip web — node_modules missing; run 'pnpm install' / task 0003)"
fi

[[ $fail -eq 0 ]] && echo "check-fast: OK" || { echo "check-fast: FAILED"; exit 1; }
