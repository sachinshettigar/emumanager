#!/usr/bin/env bash
# End-to-end smoke run: build the debug app, then drive it through tauri-driver + WebdriverIO.
#
# tauri-driver has NO macOS support (https://v2.tauri.app/develop/tests/webdriver/), so on a Mac
# this prints the manual checklist pointer and exits 0 — it never fails a nightly on a Mac.
#
# NOT part of `just validate`. Run by `just e2e` and the CI `e2e` job.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${OSTYPE:-}" == darwin* || "$(uname -s)" == "Darwin" ]]; then
  echo "E2E via tauri-driver is Linux/Windows only (no macOS support upstream)."
  echo "On a Mac, run the manual checklist: docs/playbooks/macos-e2e-checklist.md"
  exit 0
fi

command -v tauri-driver >/dev/null 2>&1 || {
  echo "tauri-driver not found. Install it: cargo install tauri-driver --locked" >&2
  exit 1
}

echo "==> building the debug app (pnpm tauri build --debug --no-bundle)"
pnpm tauri build --debug --no-bundle

echo "==> installing e2e deps (e2e/)"
cd e2e
if [[ -f package-lock.json ]]; then
  npm ci
else
  npm install
fi

echo "==> running the smoke spec"
npx wdio run ./wdio.conf.ts
