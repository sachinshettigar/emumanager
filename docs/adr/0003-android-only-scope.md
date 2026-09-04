# ADR 0003: Android only — iOS simulators are out of scope

- Status: accepted
- Date: 2026-09-04
- Deciders: project owner

## Context

The original idea covered Android emulators *and* iOS simulators on all three desktop OSes.

Apple's iOS Simulator ships only inside Xcode, Xcode runs only on macOS, and the iOS SDK /
Simulator license restricts use to Apple hardware. There is no legal or technical way to create
or run iOS simulators on Windows or Linux, and even on macOS the Xcode dependency (multi-GB,
Apple ID, non-redistributable) breaks the "zero external setup" goal.

## Decision

v1 scope is **Android emulators only**, on Windows, Linux, and macOS. No iOS, no `simctl`, no
Apple/Xcode handling anywhere in the codebase. The `Provider` trait is kept general so a
remote-Mac or cloud provider could be added later without reshaping the core.

## Consequences

- Easier: one provider implementation; no macOS-only compilation paths; the "no IDE, no external
  deps" promise is actually achievable (Android needs only Google's redistributable command-line
  tools).
- Harder / lost: users wanting a single tool for both platforms are not served in v1.
- The UI shows no platform switcher; profiles are `platform: "android"` and an import of any other
  platform is rejected with a clear message.

## Alternatives considered

- **Android everywhere + iOS only when running on macOS** — viable but doubles surface area and
  ships a worse macOS story (Xcode). Deferred.
- **Android + iOS via a remote/cloud Mac** — real cost and complexity; revisit post-1.0 behind the
  existing `Provider` trait.
