---
id: "0028"
title: "Unsigned installer config + updater + release.yml"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-07"
updated: "2026-09-07"
---

## Goal

Make a tag produce **unsigned** installers for all three OSes plus signature-verified updater
artifacts. Per ADR 0007: no OS code signing, Ed25519 updater key only. The workflow +
`tauri.conf.json` land now, `actionlint`-clean; the actual CI run stays blocked on GitHub billing.

## Context / links

- ADR: `docs/adr/0007-ship-unsigned-v1.md`
- `MILESTONES.md` M6 bullets 1, 2, 6
- `src-tauri/tauri.conf.json` — `bundle.active` is `false`, no updater block
- `.github/workflows/ci.yml` (the pattern), `.github/actions/setup/action.yml` (Linux tauri deps)
- Tauri v2 updater: <https://v2.tauri.app/plugin/updater/>

## Scope — files this task may touch

- `src-tauri/tauri.conf.json` (`bundle.active`/`targets`, `bundle.createUpdaterArtifacts`,
  `plugins.updater` with the pubkey + endpoints, NSIS/deb bits, `bundle.icon` sanity)
- `src-tauri/Cargo.toml` (`tauri-plugin-updater`), `src-tauri/src/lib.rs` (register the plugin),
  `src-tauri/capabilities/default.json` (updater permission — but see task `0029`/audit; keep minimal)
- `.github/workflows/release.yml` (new — tag-triggered matrix; `tauri-apps/tauri-action` or a
  manual `pnpm tauri build`; upload installers + `latest.json` + `.sig` to the GitHub Release)
- `.github/keys/` note or `docs/playbooks/` — how the Ed25519 keypair is generated + where the
  private key goes (CI secret `TAURI_SIGNING_PRIVATE_KEY` + `..._PASSWORD`); **the private key is
  never committed**
- `README.md` (an "Install" section: the unsigned first-run steps per OS)
- `justfile` (a `just package` that runs `pnpm tauri build` locally), `package.json` mirror
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `tauri.conf.json`: `bundle.active = true`, `bundle.targets = "all"`,
      `bundle.createUpdaterArtifacts = true`, `bundle.icon` list incl. `icon.icns` / `icon.ico`,
      `category` / descriptions, `linux.deb.depends`, `windows.nsis.installMode = "currentUser"`.
      **No** `bundle.macOS.signingIdentity` / `bundle.windows.certificateThumbprint` (ADR 0007).
- [x] `plugins.updater`: a real `pnpm tauri signer generate` **public** key committed;
      `endpoints = ["…/releases/latest/download/latest.json"]` for `sachinshettigar/emumanager`.
- [x] `tauri-plugin-updater = "2"` added; `.plugin(tauri_plugin_updater::Builder::new().build())`
      registered in `run()`; `capabilities/default.json` → `["core:default", "updater:default"]`
      (no wildcard).
- [x] `.github/workflows/release.yml` (new): `on: push: tags: ["v*"]`; matrix
      `macos-latest` ×2 (`aarch64` + `x86_64` targets) / `ubuntu-latest` / `windows-latest`;
      `tauri-apps/tauri-action@v0` builds unsigned installers + signs the updater artifacts with
      `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]` secrets; drafts a pre-release. `actionlint` clean.
- [x] `just package` (= `pnpm tauri build`) — ran it: produced `EmuManager.app` (16 MB),
      `EmuManager_0.1.0_aarch64.dmg` (**6.8 MB** — well under spec §6's 20 MB), and a signed
      `EmuManager.app.tar.gz` + `.sig` updater pair.
- [x] `README.md` "Install (pre-release)" section — macOS right-click → Open, Windows SmartScreen
      → Run anyway, Linux `chmod +x`. `docs/playbooks/release-signing.md` — keypair + secrets +
      `git tag` flow.
- [x] `just validate` green (no CI run — the workflow is written + `actionlint`-clean only).
- [x] `.gitignore` blocks `*.key` / `emumanager-updater.key*`; the private key stays in the
      session scratchpad, never committed. `docs/adr/0007-ship-unsigned-v1.md` written.

## Validate

```
just validate
actionlint .github/workflows/release.yml
just package   # local, manual — note the artifact + size in Notes
```

## Notes / findings

### The updater key

Generated with `pnpm tauri signer generate --ci -w <scratch>/emumanager-updater.key`. The
**public** key is committed in `tauri.conf.json`:
`dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhBMDI5QUEyN0I3MkI1RUUK…`. The **private**
key stays in the session scratchpad and is not persisted — **the release manager must regenerate
their own keypair before the first real `v*` tag** (`docs/playbooks/release-signing.md`), replace
the pubkey, and set the two CI secrets. Until then the committed pubkey is a build-valid
placeholder: `release.yml` will still produce installers, but updater `.sig` files won't verify
against a key anyone holds. Fine for pre-1.0.

### `tauri-action`, not hand-rolled

`tauri-apps/tauri-action@v0` does the `pnpm install` → `pnpm build` → `cargo build --release` →
bundle → upload-to-Release in one step, and understands the `TAURI_SIGNING_*` env for the updater
signature. A hand-rolled `pnpm tauri build` + manual `gh release upload` would re-implement that.

### macOS matrix is two entries

`macos-latest` runs twice with `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`
so both Apple Silicon and Intel `.dmg`s are produced (spec §6, "Apple Silicon + Intel"). Ubuntu
and Windows are single native builds.

### Local `just package`

`pnpm tauri build` on this Mac (arm64) needed nothing beyond what's already installed. Output in
`target/release/bundle/`: `.app` 16 MB, `aarch64.dmg` **6.8 MB**, plus the `.tar.gz` + `.sig`
updater pair (signed because `TAURI_SIGNING_PRIVATE_KEY` was exported for the run). `cargo`'s
`[profile.release]` already has `strip = true` + `lto = "thin"`.

### Icons

`pnpm tauri icon src-tauri/icons/icon.png` regenerated the set and added `icon.icns` / `icon.ico`
(needed for `.dmg` / NSIS). Removed the `android/` + `ios/` + MS-Store `Square*`/`StoreLogo` PNGs
it also emitted — desktop app, not mobile / MSIX.

### CI secrets `release.yml` reads

`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (both repo secrets), plus the
automatic `GITHUB_TOKEN`. No Apple/Windows signing secrets — unsigned by design (ADR 0007).

### Still CI-gated

`release.yml` cannot *run* until the account's GitHub Actions billing block is cleared (same block
that's held M0's `ci.yml` from a green run). The workflow is written and `actionlint`-clean so
that's a one-secret + one-tag operation once unblocked.
