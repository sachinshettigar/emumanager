# ADR 0008: Reuse a JDK if present, otherwise download Temurin 17

- Status: accepted (supersedes [ADR 0006](0006-require-system-jdk.md))
- Date: 2026-10-07
- Deciders: project owner (via task 0041)

## Context

ADR 0006 made a system JDK 17+ a hard prerequisite and deferred bundling "if real users hit this
wall". They did: on a fresh machine with no Java, setup stopped with "install a JDK and set
`JAVA_HOME`" — the opposite of the product promise (a person who has never touched Android tooling
can create an emulator). Users who already have Java in a non-`PATH` place (Android Studio's
bundled JBR, Homebrew, `/usr/lib/jvm`, Adoptium folders) were also told to install one.

## Decision

`toolchain::jdk::ensure` runs before any `sdkmanager` work and resolves a JDK in this order,
accepting only one that answers `java -version` with 17+:

1. the JDK recorded by a previous run (`<data_dir>/jdk/java_home`),
2. `JAVA_HOME`,
3. `java` on `PATH` (home read from `-XshowSettings:properties`),
4. well-known install locations per OS (incl. Android Studio's JBR),
5. otherwise download **Eclipse Temurin 17** into `<data_dir>/jdk/temurin-17`. The archive URL and
   SHA-256 come from the Adoptium API (`/v3/assets/latest/17/hotspot`), the download is
   checksum-verified, `.tar.gz` is unpacked with the system `tar` (preserves symlinks and exec
   bits), `.zip` (Windows) in-process.

The chosen home is recorded and every `sdkmanager` / `avdmanager` call gets `JAVA_HOME` pinned to
it, so a stale or too-old `JAVA_HOME` in the user's shell can't break the tools.

## Consequences

- Zero-setup holds on a machine with nothing installed; existing JDKs are reused, not duplicated.
- One extra ~190 MB download on machines with no Java (once; kept in the app data dir).
- A new network dependency on `api.adoptium.net` / GitHub releases — same class as Google's SDK
  repository, and failures surface as a normal download error.
- Not covered: Windows on ARM (Temurin 17 publishes no build there); the error says to install a
  JDK manually.
