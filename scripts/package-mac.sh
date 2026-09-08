#!/usr/bin/env bash
# Build the macOS installers locally. Runs on macOS only.
#
# Produces, per architecture, under target/<triple>/release/bundle/:
#   macos/Emulator Studio.app          the app bundle
#   dmg/Emulator Studio_<ver>_<arch>.dmg   the disk image
#   macos/Emulator Studio.app.tar.gz + .sig   updater artifacts (only if a signing key is set)
#
# Everything is UNSIGNED / un-notarized (ADR 0007) — first-run steps are in the README.
#
# Usage:
#   scripts/package-mac.sh              # native arch only (fast)
#   scripts/package-mac.sh --universal  # arm64 + x86_64 (adds the x86_64 target, slower)
set -euo pipefail
cd "$(dirname "$0")/.."

[[ "$(uname -s)" == "Darwin" ]] || { echo "This script builds macOS installers and must run on macOS." >&2; exit 1; }

BOTH_ARCHES=false
[[ "${1:-}" == "--universal" || "${1:-}" == "--both" ]] && BOTH_ARCHES=true

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  echo "note: TAURI_SIGNING_PRIVATE_KEY is not set — the .app/.dmg build fine, but the"
  echo "      updater .tar.gz won't be signed (auto-update won't verify). See"
  echo "      docs/playbooks/release-signing.md."
  echo
fi

build_one() {
  local target="$1"
  echo "==> pnpm tauri build --target ${target}"
  rustup target add "$target" >/dev/null 2>&1 || true
  # `|| true`: the updater-signing step exits non-zero when no key is set, *after* the .app/.dmg
  # are already written — which is fine for a local unsigned build.
  pnpm tauri build --target "$target" || true
  echo
  echo "artifacts for ${target}:"
  find "target/${target}/release/bundle" -maxdepth 2 \
    \( -name "*.dmg" -o -name "*.app" -o -name "*.app.tar.gz" -o -name "*.sig" \) \
    -exec du -sh {} \; 2>/dev/null || true
  echo
}

build_one aarch64-apple-darwin
if $BOTH_ARCHES; then
  build_one x86_64-apple-darwin
fi

echo "done. Ship the .dmg files; keep the .app.tar.gz + .sig for the updater feed."
