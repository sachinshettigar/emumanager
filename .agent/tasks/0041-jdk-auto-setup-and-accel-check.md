---
id: "0041"
title: "Auto-resolve/download Java + trust the emulator's own acceleration check"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-10-07"
updated: "2026-10-08"
---

## Goal

Two blockers hit on a fresh Windows work PC:

1. **No Java 17** → setup stopped. Assume nothing is installed: find and reuse any JDK 17+
   (any OS), else download Temurin 17 ourselves. See ADR 0008.
2. **"Hardware virtualization is turned off" blocked Launch** although the same machine ran
   emulators from Android Studio. Root cause: on Windows `VirtualizationFirmwareEnabled` reads
   `False` whenever a hypervisor (Hyper-V/VBS/WSL2 — standard on managed PCs) owns the CPU
   feature, and the UI hard-disabled Launch on that guess.

Also (user ask): make **Install all** prominent, below the component list.

## Scope — files touched

- `crates/emu-core/src/toolchain/jdk.rs` (new) + `bootstrap.rs` / `uninstall.rs` / `mod.rs`
- `crates/emu-core/tests/fixtures/adoptium-latest-17-linux-x64.json`
- `crates/emu-android/src/provider.rs` — `with_java`, `accel_check`, `parse_accel_check`;
  fixture `emulator-accel-check-macos.txt`
- `crates/emu-host/src/{signals,report,probe}.rs`, `crates/emu-core/src/model/host.rs` (`AccelCheck`)
- `src-tauri/src/commands/{host,toolchain}.rs`
- `src/routes/{Dashboard,EmulatorDetail,Dependencies}.tsx` (+ tests)
- `docs/adr/0008-resolve-or-download-jdk.md`, ADR 0006 status

## Acceptance criteria

- [x] JDK resolved in order remembered → `JAVA_HOME` → `PATH` → well-known dirs → download.
- [x] Download is SHA-256 verified; Windows zip + unix tar.gz paths unit-tested; layout checked
      against a real Temurin 17 macOS tarball.
- [x] `JAVA_HOME` pinned on every `sdkmanager`/`avdmanager` call (bootstrap, install, create,
      delete, list, uninstall).
- [x] Windows probe treats a present hypervisor as virtualization on.
- [x] `emulator -accel-check` overrides the guesses; only it can produce `cannotRun`.
- [x] Launch is never disabled by a host verdict; a warning with the reason is shown.
- [x] Install all: full-width primary button below the list, with total size.
- [ ] Verified on a real Windows machine (user to confirm with the released build).

## Validate

`just validate`
