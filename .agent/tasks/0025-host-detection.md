---
id: "0025"
title: "emu-host — NativeHostProbe: virtualization / accelerator / disk / RAM → HostReport"
milestone: "M5"
status: "review"
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

- [x] `HostSignals` (`crates/emu-host/src/signals.rs`): `os` / `arch` strings, `virtualization:
      Virtualization`, `accel: AccelSignal` (`Ready` / `NoPermission` / `Missing` / `Disabled` /
      `Unknown`), `ram_bytes`, `disk_free_bytes`. (No separate `cpu_supports_virt` — the accelerator
      probe result carries enough; a `Missing` accelerator drives the verdict.)
- [x] `build_report(&HostSignals) -> HostReport` (`report.rs`) — pure, no `#[cfg]`. Kind by OS
      (Linux→Kvm, Windows→Whpx, macOS→Hvf). `Verdict`:
  - `CannotRun` — `DisabledInFirmware`; RAM < 4 GB; disk-free < 8 GB; accelerator `Missing`.
  - `Degraded` — accelerator `NoPermission` / `Disabled`; RAM < 8 GB; disk-free < 25 GB.
  - `CanAccelerate` otherwise (and `Unknown` signals never hard-block).
  - `fixes[]`: `enable-virtualization` (non-scriptable, reboot) for `DisabledInFirmware`;
    `add-kvm-group` (scriptable, no reboot) / `install-kvm` (non-scriptable) for KVM;
    `enable-whpx` (scriptable, reboot) / `enable-aehd` for WHPX. Each `description` cites the real
    command.
- [x] `NativeHostProbe` (`probe.rs`) implements `HostProbe`: RAM/disk via `sysinfo` (the disk whose
      mount point is the longest prefix of `data_dir`); `platform_probe()` per OS — macOS
      `sysctl -n kern.hv_support`; Linux `/dev/kvm` existence + read/write open (→ `NoPermission`
      on `EACCES`); Windows `powershell` `Get-WindowsOptionalFeature` + `VirtualizationFirmwareEnabled`
      (best-effort, **untested on this macOS dev machine** — flagged in the module + Notes). Missing
      → `Unknown`.
- [x] 8 `build_report` unit tests: healthy Mac → `CanAccelerate`/no fixes; CI-runner no-virt →
      `CannotRun` + `enable-virtualization`; KVM `NoPermission` → `Degraded` + `add-kvm-group`;
      WHPX `Missing` → `CannotRun` + `enable-whpx` (reboot); low-RAM → `Degraded`; too-little-RAM /
      too-little-disk → `CannotRun`; all-`Unknown` → not a block.
- [x] `#[ignore]` real-machine test — ran it: `os=macos arch=aarch64 virt=Enabled accel=Hvf/Ok
      ram=17179869184B disk_free≈163GB verdict=CanAccelerate`.
- [x] `just check-fast` then `just validate` green (149 rust tests).

## Validate

```
cargo test -p emu-host --all-features
just validate
```

## Notes / findings

### `sysinfo` for RAM + disk

`sysinfo 0.36` (`default-features = false`, `system` + `disk` only — 0.36 is the last release for
rustc 1.82; 0.39 needs 1.95). One crate covers all three OSes without a per-OS syscall of our own,
and `#![forbid(unsafe_code)]` stays. `mem_and_disk` picks the disk whose `mount_point()` is the
longest prefix of `data_dir` (so a bind-mounted data dir reports the right volume).

### KVM without `unsafe`

Linux group membership is checked implicitly: `OpenOptions::new().read(true).write(true).open("/dev/kvm")`
→ `Ok` = `Ready`, `PermissionDenied` = `NoPermission` (the "not in the `kvm` group" case),
`NotFound` = `Missing`. No `libc`, no ioctls — the emulator opens `/dev/kvm` the same way, so this
is exactly the check that matters.

### Real probe on this machine

`cargo test -p emu-host real_inspect_succeeds -- --ignored --nocapture`:
`os=macos arch=aarch64 virt=Enabled accel=Hvf/Ok ram=16 GiB disk_free≈163 GB verdict=CanAccelerate`.

### RAM / disk floors

`RAM_FLOOR 4 GB` / `RAM_LOW 8 GB`, `DISK_FLOOR 8 GB free` / `DISK_LOW 25 GB free`. The floors are
"an emulator genuinely won't work" (Google's own minimum is ~4 GB RAM; a modern system image is
1–2 GB unpacked plus the AVD's userdata); the low thresholds are "you'll hit trouble soon". A
signal of `0` (couldn't read) is skipped, never treated as "below the floor".

### `Unknown` per OS

macOS: `virtualization` is `Unknown` when `kern.hv_support` is `0`/unreadable (Apple has no
firmware toggle, so `DisabledInFirmware` never applies). Linux: `virtualization` stays `Unknown`
whenever `/dev/kvm` is absent (module-not-loaded vs firmware-off vs no-CPU-support aren't cheaply
distinguishable). Windows: the whole probe is best-effort via `powershell` and **has not been run**
on a real Windows host — the mapping (`enabled`→Ready, `disabled`→Missing, `false`→DisabledInFirmware)
is from the documented cmdlet output; verify when a Windows runner is available (M6).

### `AcceleratorKind::Aehd` never chosen by `accelerator_kind_for`

Windows always maps to `Whpx`; the AMD-only AEHD path surfaces as a `Fix` (`enable-aehd`) when the
WHPX signal is `NoPermission` (a proxy for "WHPX present but this AMD CPU can't use it"). A cleaner
AMD-vs-Intel split is a follow-up if it matters.
