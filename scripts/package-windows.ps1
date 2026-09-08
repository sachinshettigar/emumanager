# Build the Windows installers. RUN THIS ON WINDOWS (PowerShell) — Tauri's Windows
# bundlers (NSIS / WiX) and the MSVC + WebView2 toolchain do not run on macOS or Linux.
#
# Produces under src-tauri\target\release\bundle\:
#   nsis\Emulator Studio_<ver>_x64-setup.exe   the NSIS installer
#   msi\Emulator Studio_<ver>_x64_en-US.msi    the MSI (if WiX is available)
#   Emulator Studio_<ver>_x64-setup.nsis.zip + .sig   updater artifacts (only if a signing key is set)
#
# Everything is UNSIGNED (ADR 0007). First-run: SmartScreen -> "More info" -> "Run anyway".
#
# Prereqs (one-time):
#   - Rust (stable) with the MSVC toolchain, Node 20+, pnpm
#   - "Desktop development with C++" workload (VS Build Tools) for the MSVC linker
#   - WebView2 runtime (usually present on Win10/11)
#
# Usage (from the repo root, in PowerShell):
#   pwsh scripts/package-windows.ps1

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

if (-not $Env:TAURI_SIGNING_PRIVATE_KEY) {
  Write-Host "note: TAURI_SIGNING_PRIVATE_KEY is not set - the .exe/.msi build fine, but the"
  Write-Host "      updater .zip won't be signed. See docs/playbooks/release-signing.md."
  Write-Host ""
}

Write-Host "==> pnpm tauri build"
# The updater-signing step fails (non-zero) when no key is set, AFTER the installers are written.
& pnpm tauri build
Write-Host ""

Write-Host "artifacts:"
Get-ChildItem -Recurse "src-tauri/target/release/bundle" -Include *.exe, *.msi, *.zip, *.sig |
  Select-Object FullName, @{n = "SizeMB"; e = { [math]::Round($_.Length / 1MB, 1) } } |
  Format-Table -AutoSize

Write-Host "done. Ship the setup .exe (and/or .msi); keep the .zip + .sig for the updater feed."
