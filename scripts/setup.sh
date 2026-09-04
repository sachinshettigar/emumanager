#!/usr/bin/env bash
# Install toolchains + dev deps + git hooks. Idempotent. Safe to re-run.
set -euo pipefail
have() { command -v "$1" >/dev/null 2>&1; }

echo "==> Rust"
have rustup || { echo "Install Rust: https://rustup.rs"; exit 1; }
rustup show active-toolchain || rustup toolchain install stable
rustup component add rustfmt clippy
for c in cargo-nextest cargo-deny cargo-machete cargo-llvm-cov sqlx-cli typos-cli; do
  have "${c%%-cli}" || have "$c" || cargo install "$c" --locked || true
done

echo "==> Node / pnpm"
have pnpm || { echo "Install pnpm: https://pnpm.io/installation"; exit 1; }
[[ -f package.json ]] && pnpm install || echo "   (no package.json yet — pre task 0003)"

echo "==> CLI tools (best effort)"
# markdownlint-cli2 + knip ship as dev-dependencies (pnpm install above).
for t in just actionlint gitleaks lychee; do
  if have "$t"; then continue; fi
  if have brew; then brew install "$t" || true; fi
  have "$t" || echo "   missing: $t  (brew/scoop/apt, or a GitHub release binary)"
done

echo "==> git hooks"
if have lefthook; then lefthook install; else echo "   install lefthook then re-run: https://lefthook.dev"; fi

echo "==> done. Try: just validate"
