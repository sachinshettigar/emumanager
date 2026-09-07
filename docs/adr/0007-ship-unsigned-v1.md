# ADR 0007: Ship v1 installers unsigned; keep updater artifacts signature-verified

- Status: accepted
- Date: 2026-09-07
- Deciders: project owner

## Context

`docs/spec.md` §6 (Non-functional / Security) requires the v1 installers to be **code-signed +
notarized on macOS** and **code-signed on Windows**, and §7's success criteria lean on a clean
first-run experience. M6's Definition of Done was written around that: "installing the macOS build
shows no Gatekeeper warning".

Code signing needs real, paid credentials held as CI secrets:

- macOS: an Apple Developer Program membership ($99/yr), a "Developer ID Application" certificate,
  and an app-specific password / API key for `notarytool`.
- Windows: an OV or EV code-signing certificate from a CA (~$200–500/yr; EV needs a hardware
  token).

The project owner does not want to obtain or manage those credentials for the current release and
is comfortable distributing the app **unsigned**, with users doing the standard OS override on
first launch.

Separately, Tauri's **updater** signing is unrelated to OS code signing: it's a self-generated
Ed25519 keypair (`tauri signer generate`) whose public key ships in `tauri.conf.json` and whose
private key signs each update artifact. It costs nothing and makes auto-updates tamper-evident.

## Decision

**v1 ships unsigned OS installers** (`.dmg`, `.msi`/NSIS, `.AppImage`, `.deb`) and **does not
notarize**. First-run friction is accepted and documented:

- macOS: Gatekeeper blocks a double-click of an unsigned `.dmg` app. Users right-click the app →
  **Open** → **Open** once (or `xattr -dr com.apple.quarantine` for the CLI-inclined). The README
  and the download page must spell this out.
- Windows: SmartScreen shows "Windows protected your PC". Users click **More info → Run anyway**.
- Linux: no signing expectation; `.AppImage` just needs `chmod +x`.

**The Tauri updater stays fully signed** with an Ed25519 key. `release.yml` generates
`latest.json` + per-artifact `.sig` files; the app verifies them before applying an update. The
updater private key lives in a CI secret (`TAURI_SIGNING_PRIVATE_KEY` + password); the public key
is committed in `tauri.conf.json`. This is the one signing credential the project keeps.

`docs/spec.md` §6 and §7 are amended to match; M6's DoD drops "no Gatekeeper warning" and
"signed/notarized", keeping "unsigned installers build for all three OSes in CI" and "the app
auto-updates from the previous pre-release (signature-verified)".

Adopting OS code signing later is a config + secrets change (`tauri.conf.json` `bundle.macOS` /
`bundle.windows`, three or four CI secrets) — no code rewrite. Tracked here, not silently dropped.

## Consequences

- **Easier:** no paid credentials, no notarization step in CI, M6 stops being blocked on a
  purchase. The updater keypair is free and self-serve.
- **Harder / accepted:** every first install has an OS scare dialog the user must click through;
  the download docs carry that instruction. Enterprise deploy tools that reject unsigned binaries
  won't take it as-is.
- **Unchanged:** least-privilege Tauri capabilities, the separate audited `emu-helper`, and the
  updater's own signature verification all still apply — dropping OS signing doesn't loosen any of
  those.
- The E2E-in-CI line of M6's DoD is still gated on GitHub Actions billing (a separate, pre-existing
  block — see `MILESTONES.md` M0), not on this decision.
