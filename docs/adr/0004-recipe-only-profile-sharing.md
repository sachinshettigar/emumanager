# ADR 0004: Profile sharing is recipe-only

- Status: accepted
- Date: 2026-09-04
- Deciders: project owner

## Context

Users want to share an emulator setup. Two models: share a **recipe** (a small description that
the other machine resolves and rebuilds) or share a **snapshot** (the actual AVD disk/state
bytes).

Snapshots are multi-GB, tied to host arch (x86_64 vs. arm64) and emulator version, and can carry
whatever the sender ran inside the emulator (privacy/security surface).

## Decision

`.emuprofile` is a **recipe only**: a small JSON document (schema
`schemas/emuprofile/v1.schema.json`) describing device profile, image coordinates
(`api`/`type`/`abi`), hardware parameters, and an optional seed section that references APK
*filenames* the importer must supply separately — never embedded binaries. It contains no SDK
components and no disk images. Import resolves requirements against the local install, downloads
what's missing from Google's servers, and creates the instance locally.

Same-arch snapshot export/import may be added later as a clearly separate, opt-in feature.

## Consequences

- Easier: tiny, reviewable, diffable, safe-to-share files; version control friendly; no arch
  coupling.
- Harder / lost: the imported emulator is *equivalent*, not *identical* — anything the sender
  installed or configured at runtime is not reproduced unless expressed in the recipe's seed
  section.
- The importer needs network + disk for whatever the recipe requires; the UI shows the
  requirement diff and sizes before doing anything.

## Alternatives considered

- **Snapshot bundles** — reproducible to the byte, but huge, arch-locked, and a data-exfiltration
  footgun. Rejected for v1.
- **Recipe + optional snapshot in one file** — confusing size/behavior; keep them separate
  features if snapshots ever land.
