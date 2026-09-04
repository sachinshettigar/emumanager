# ADR 0006: Require a system JDK 17+ instead of bundling one

- Status: accepted
- Date: 2026-09-05
- Deciders: project owner (via task 0012)

## Context

`docs/spec.md` §5.1 lists a "bundled JRE" as part of the detected/managed toolchain state, and
§8's open question leaned toward bundling (~40 MB) for true zero-setup. Task 0012 (toolchain
bootstrap) had to resolve this before `bootstrap()` could run `sdkmanager` for real.

Two facts, checked against the real tool on this machine (`java -version`, `sdkmanager --version`)
rather than assumed:

- Modern `cmdline-tools` (the version resolved by task 0010's catalog parser, `sdkmanager` 19.0)
  needs a JDK on `PATH`/`JAVA_HOME` to run at all — it's a shell/batch wrapper around a `java`
  invocation, not a self-contained binary. Google has not shipped a bundled JRE with `cmdline-tools`
  since API level tooling moved off the old standalone SDK Tools package.
- A "bundled JRE" means EmuManager would have to fetch, verify, unpack, and keep updated a full
  per-OS/per-arch JRE (Eclipse Temurin or similar) *before* it can even ask `sdkmanager` anything —
  a second, unrelated toolchain-management problem on top of the Android SDK one.

## Decision

**v1 requires a system JDK 17+** already on `PATH` or referenced by `JAVA_HOME`. `bootstrap()`
checks for one (`java -version`, parsed for the major version) before doing anything else and
fails fast with a specific, actionable error — "no JDK 17+ found on PATH or JAVA_HOME; install one
(e.g. Eclipse Temurin) and set JAVA_HOME, then retry" — instead of downloading anything and letting
`sdkmanager` fail cryptically later. `InstalledState`/`bootstrap` never bundle or manage a JRE.

Bundling a portable JRE is deferred to a follow-up task if real users hit this wall often enough to
justify it — tracked here, not silently dropped.

## Consequences

- Easier: `bootstrap()` stays scoped to what it's actually for (the Android SDK), one fewer
  download/verify/unpack pipeline to build and keep current against upstream JRE releases.
- Harder: goal 1 ("zero external setup... no terminal commands the user runs") is not fully met for
  a machine with no JDK at all — that user must install one manually before EmuManager can finish
  setup. Mitigated by making the failure immediate and specific rather than a late, confusing
  `sdkmanager` crash.
- Most target users (mobile/QA engineers, `docs/spec.md` §2) already have a JDK from other Android
  work; this mainly bites a genuinely fresh machine.

## Alternatives considered

- **Bundle a portable JRE now** (e.g. Eclipse Temurin, ~40 MB) — closer to true zero-setup, but adds
  a second fetch-verify-unpack-update pipeline (with its own catalog, checksums, and per-OS/arch
  archive selection) before M1's actual job — the Android SDK — even starts. Revisit post-v1 if the
  system-JDK requirement proves to be real friction.
