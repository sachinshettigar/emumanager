---
id: "0025"
title: "emu-host — NativeHostProbe: virtualization / accelerator / disk / RAM → HostReport"
milestone: "M5"
status: "doing"
owner: "Claude Code"
created: "2026-09-06"
updated: "2026-09-06"
---

## Goal

`emu-host` implements the `HostProbe` port: gather the raw host signals per OS, then derive a
`HostReport` with an accelerator picture, a `Verdict`, and an ordered `fixes[]`. The **derivation**
(signals → verdict → fixes) is pure and fully unit-tested; the native signal-gathering is a thin
per-OS layer behind an injected input so tests never touch real hardware.

## Context / links

- Milestones: `MILESTONES.md` M5 bullet 1 + the DoD (CI-runner-without-virt → `CannotRun`)
- Architecture: `docs/architecture.md` §2 (`emu-host` row), §5 "Host readiness / fix"
- Types already exist (`crates/emu-core/src/model/host.rs`, task `0002`): `HostReport`,
  `Virtualization`, `Accelerator{kind,status}`, `AcceleratorKind`, `AcceleratorStatus`, `Verdict`
  (`CanAccelerate` / `Degraded{reason}` / `CannotRun{reason}`), `Fix{id,title,scriptable,needs_reboot,description}`
- `HostProbe` port: `async fn inspect(&self) -> Result<HostReport>` (`crates/emu-core/src/ports.rs`)

## Scope — files this task may touch

- `crates/emu-host/src/lib.rs`, `crates/emu-host/src/signals.rs` (new — `HostSignals` struct + the
  per-OS gatherers behind `#[cfg]`), `crates/emu-host/src/report.rs` (new — `build_report(signals)`
  the pure derivation), `crates/emu-host/src/probe.rs` (new — `NativeHostProbe` impl `HostProbe`)
- `crates/emu-host/Cargo.toml` (add `sysinfo` for RAM/disk, `async-trait`; keep it small)
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [ ] `HostSignals` — the OS-independent input to the derivation: `os` / `arch` strings,
      `virtualization: Virtualization`, `cpu_supports_virt: bool`, an OS-appropriate accelerator
      probe result (e.g. `kvm_present` / `kvm_readable` / `whpx_enabled` / `aehd_installed` /
      `hvf_present`), `ram_bytes`, `disk_free_bytes`.
- [ ] `build_report(&HostSignals) -> HostReport` — pure. Picks the `AcceleratorKind` from `os`
      (Linux→Kvm, Windows→Whpx or Aehd, macOS→Hvf), maps the probe result to `AcceleratorStatus`,
      then:
  - `Verdict::CannotRun` when virtualization is `DisabledInFirmware`, or the accelerator is
    `Missing` on a platform with no software fallback we allow, or RAM/disk below a hard floor.
  - `Verdict::Degraded` when the accelerator is `NoPermission` / `Disabled`, or RAM/disk are low.
  - `Verdict::CanAccelerate` otherwise.
  - `fixes[]` matching the blockers: `enable-whpx` (`needs_reboot`), `enable-aehd`,
    `add-kvm-group` (not `needs_reboot`), plus a non-scriptable "enable virtualization in BIOS"
    when `DisabledInFirmware`. Cite each fix's real command in its `description`.
- [ ] `NativeHostProbe` implements `HostProbe`: gathers `HostSignals` for the current OS (RAM/disk
      via `sysinfo`; virtualization + accelerator via `#[cfg(target_os = …)]` helpers — cpuid on
      x86, `/dev/kvm` + `kvm` group membership on Linux, `HVF` sysctl on macOS, WHPX/AEHD checks on
      Windows), then `build_report`. Missing signals → `Unknown`, never a panic.
- [ ] Unit tests on `build_report` with fabricated `HostSignals`: the CI-runner case (no virt →
      `CannotRun` + the BIOS fix), KVM-no-permission → `Degraded` + `add-kvm-group`, WHPX-missing →
      `CannotRun` + `enable-whpx`, healthy Mac → `CanAccelerate` + no fixes, low-RAM → `Degraded`.
- [ ] One `#[ignore]` "real machine" test that runs `NativeHostProbe::inspect()` and just asserts
      it returns `Ok` with a sane `os`/`arch` — run it once for real, note the result.
- [ ] `just check-fast` then `just validate` green; `emu-host` builds on all three targets
      (`cargo check` is enough — CI does the real matrix).

## Validate

```
cargo test -p emu-host --all-features
just validate
```

## Notes / findings

(Fill in: `sysinfo` vs a lighter crate; how KVM group membership is checked without `unsafe`;
what the real `NativeHostProbe::inspect()` returned on this Mac; the RAM/disk floors chosen and
why; any signal that stays `Unknown` on a given OS.)
