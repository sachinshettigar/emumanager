# Playbook: release keys & cutting a release

EmuManager ships **unsigned** OS installers (ADR 0007). The only signing credential is the **Tauri
updater key** (Ed25519), used to sign the auto-update artifacts so the app can verify them.

## One-time: generate the updater keypair

```bash
pnpm tauri signer generate -w emumanager-updater.key
```

This prints a **public key** and writes a **private key** to `emumanager-updater.key`
(`.key.pub` alongside).

1. Put the public key into `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`.
   (A build-valid placeholder is committed today — replace it with yours before the first `v*`
   tag, or updates signed in CI won't verify on installed apps.)
2. Add two **repo secrets** (Settings → Secrets and variables → Actions):
   - `TAURI_SIGNING_PRIVATE_KEY` — the full contents of `emumanager-updater.key`.
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — its password (an empty string if you generated it
     without `-p`).
3. **Never commit `emumanager-updater.key`.** It's in `.gitignore`; `gitleaks` (in `just validate`)
   is the backstop.

Losing the private key means you can't sign further updates — installed apps then can't auto-update
until they're manually reinstalled with a new key. Back it up somewhere safe (a password manager).

## Cut a release

```bash
# bump src-tauri/tauri.conf.json "version" and Cargo workspace version together, commit
git tag v0.2.0
git push origin v0.2.0
```

`.github/workflows/release.yml` then builds unsigned `.dmg` (arm64 + x64) / `.msi`+NSIS /
`.AppImage`+`.deb` on the matrix, signs the updater artifacts with the secret key, and attaches
everything plus `latest.json` to a **draft pre-release**. Review it, then publish.

> This workflow does not run until the account's GitHub Actions billing block is cleared (see
> `MILESTONES.md` M0). Until then, `just package` builds installers locally.

## Local build

```bash
just package        # pnpm tauri build — unsigned, current OS/arch only
```

Output lands in `src-tauri/target/release/bundle/`.

## Adding OS code signing later

Not a rewrite — set `bundle.macOS.signingIdentity` / `bundle.windows.certificateThumbprint` in
`tauri.conf.json` and add the cert + notarization secrets to `release.yml`'s `env`. ADR 0007 has
the detail.
