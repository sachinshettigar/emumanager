# Playbook: building installers locally

All installers are **unsigned / un-notarized** (ADR 0007). First-run instructions per OS are in
the [README](../../README.md#install-pre-release). The only signing credential is the Tauri
**updater key** — see [`release-signing.md`](release-signing.md); without it the `.app`/`.dmg`/
`.exe` still build, only the updater feed artifacts (`.tar.gz` / `.zip` + `.sig`) go unsigned.

## What can I build where?

| Target | On macOS | On Windows | On Linux | CI |
| --- | --- | --- | --- | --- |
| macOS `.app` + `.dmg` (arm64) | **yes** (native) | no | no | `macos-latest` |
| macOS `.dmg` (x86_64) | yes (`--universal`, cross target) | no | no | `macos-latest` |
| Windows `.exe` (NSIS) + `.msi` | **no** | **yes** | no¹ | `windows-latest` |
| Linux `.AppImage` + `.deb` | no | no | **yes** | `ubuntu-latest` |

¹ There is an experimental `cargo-xwin` path to cross-compile a Windows *binary* from Linux, but
Tauri's Windows **bundlers** (NSIS / WiX) are Windows-only, so you'd get no installer. Not worth it.

**So from a Mac you can produce the macOS installers and nothing else.** Windows and Linux
installers need their own OS — a real machine, a VM (Parallels / UTM / Lima), or CI.

## Commands

```sh
just package-mac              # arm64 .app + .dmg (fast)
just package-mac --universal  # + x86_64 .dmg (adds the rust target, slower)
just package-windows          # ON WINDOWS ONLY (PowerShell): .exe + .msi
```

Artifacts land under `target/<triple>/release/bundle/` (macOS) or
`src-tauri/target/release/bundle/` (Windows):

- `macos/Emulator Studio.app` · `dmg/Emulator Studio_<ver>_<arch>.dmg`
- `nsis/Emulator Studio_<ver>_x64-setup.exe` · `msi/Emulator Studio_<ver>_x64_en-US.msi`
- `*.app.tar.gz` / `*-setup.nsis.zip` + `*.sig` — the updater feed (signed only when the key is set)

## The all-OS build: CI

`.github/workflows/release.yml` builds every OS on a pushed `v*` tag and attaches the artifacts to
a draft GitHub Release, with the updater feed signed from repo secrets. That is the intended path
for a real release — it is currently blocked only by the account's GitHub Actions billing (see
`MILESTONES.md` M0). Until that's lifted, hand-build per OS with the commands above and upload to a
Release manually.

## Known local hiccups (macOS)

- **`bundle_dmg.sh` fails intermittently** — usually a stale mounted `Emulator Studio` volume or a
  Finder-automation permission prompt. Unmount any leftover volume in Finder and re-run; the
  `.app` is written before the `.dmg` step so it's already there.
- **"A public key has been found, but no private key"** at the end of the build — expected when
  `TAURI_SIGNING_PRIVATE_KEY` isn't set. The `.app` and `.dmg` are fine; only the updater
  `.tar.gz` wasn't signed.
