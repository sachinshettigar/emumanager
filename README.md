# Emulator Studio

A cross-platform desktop app to **create, launch, track, and share Android emulators** — no
Android Studio, no manual `sdkmanager` commands. Windows, Linux, macOS.

> Status: **pre-M0** (scaffolding). See [`PROGRESS.md`](PROGRESS.md).

## What it does

- Bootstraps the Android SDK it needs (command-line tools, platform-tools, emulator, system
  images) into its own data directory — nothing installed system-wide, no IDE.
- Browse device models + API levels, create an emulator, launch it, watch its logs.
- Tracks every emulator you've created and its running state; relaunch, stop, wipe.
- Export an emulator as a portable `.emuprofile` recipe; import one and it resolves + downloads
  what's missing and re-creates the instance locally. **Recipes carry no SDK or image bytes.**

Scope is Android only — iOS simulators cannot run off macOS, so they are out of scope
([ADR 0003](docs/adr/0003-android-only-scope.md)).

## Install (pre-release)

Downloads on the [Releases page](https://github.com/sachinshettigar/emumanager/releases) are
**unsigned** ([ADR 0007](docs/adr/0007-ship-unsigned-v1.md)) — your OS will warn on first launch.
Once past that, the app auto-updates (the updater *is* signature-verified).

- **macOS** (`.dmg`): open it, drag the app to Applications, then **right-click the app → Open →
  Open**. (A plain double-click is blocked by Gatekeeper the first time only.)
- **Windows** (`.msi` / setup `.exe`): run it; on the SmartScreen prompt click **More info → Run
  anyway**.
- **Linux** (`.AppImage`): `chmod +x` the `.AppImage` and run it. `.deb` also provided.

## Build & run

Prerequisites: Rust (stable), Node 20+, `pnpm`, `just`. Then:

```bash
just setup     # install remaining toolchains + dev deps (idempotent)
just dev       # run the app
just validate  # run the full check gate (what CI runs)
just package   # build an unsigned installer for this OS (pnpm tauri build)
```

Platform notes: emulator acceleration needs KVM (Linux), the Windows Hypervisor Platform
(Windows), or Hypervisor.framework (macOS). The app detects and guides; a one-time elevated
helper handles the scriptable parts.

## Developing with AI tools

This repo is built to be developed by AI coding agents and to survive switching between them
(Claude Code, Cursor, Gemini, Antigravity, …). The contract for any agent or human is
[`AGENTS.md`](AGENTS.md). Every tool-specific config file is a stub that points there.

- Plan & milestones: [`MILESTONES.md`](MILESTONES.md)
- Architecture: [`docs/architecture.md`](docs/architecture.md)
- Decisions: [`docs/adr/`](docs/adr/)
- Working state (tasks, journal, machine-readable progress): [`.agent/`](.agent/)
- Recipes ("skills"): [`docs/playbooks/`](docs/playbooks/)

## License

TBD before first public release.
