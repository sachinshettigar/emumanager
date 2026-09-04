#!/usr/bin/env bash
# Fails if crates/emu-core gains a dependency on `tauri` (AGENTS.md §6 rule 1).
set -euo pipefail

manifest="crates/emu-core/Cargo.toml"
if [[ ! -f "$manifest" ]]; then
  echo "emu-core-no-tauri: $manifest not found yet (pre task 0001) — skipping."
  exit 0
fi

if grep -Eiq '^[[:space:]]*tauri([-_][a-z]+)?[[:space:]]*=' "$manifest"; then
  echo "FAIL: $manifest declares a tauri dependency. emu-core must stay tauri-free."
  exit 1
fi

if command -v cargo >/dev/null 2>&1; then
  if cargo tree -p emu-core -e normal 2>/dev/null | grep -Eq '(^| )tauri v'; then
    echo "FAIL: emu-core pulls tauri transitively:"
    cargo tree -p emu-core -e normal -i tauri 2>/dev/null || true
    exit 1
  fi
fi

echo "emu-core-no-tauri: ok"
