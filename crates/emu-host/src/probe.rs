//! [`NativeHostProbe`] — gathers [`HostSignals`] for the current OS, then [`build_report`].
//!
//! The RAM/disk numbers come from `sysinfo`. Virtualization + accelerator detection is per-OS,
//! behind `#[cfg(target_os = …)]`; anything it can't determine is left `Unknown` (never a panic,
//! never a hard block). The pure part is [`crate::report::build_report`] — see it for the
//! verdict/fixes logic and its tests.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use emu_core::model::host::{HostReport, Virtualization};
use emu_core::ports::HostProbe;
use emu_core::Result;

use crate::report::build_report;
use crate::signals::{AccelSignal, HostSignals};

/// The real [`HostProbe`]. `data_dir` picks the volume whose free space is reported.
pub struct NativeHostProbe {
    data_dir: PathBuf,
}

impl NativeHostProbe {
    /// Probe free disk on the volume containing `data_dir`.
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Gather the raw signals for whatever OS we're compiled for.
    #[must_use]
    pub fn gather(&self) -> HostSignals {
        let (ram_bytes, disk_free_bytes) = mem_and_disk(&self.data_dir);
        let (virtualization, accel) = platform_probe();
        HostSignals {
            os: os_family().to_string(),
            arch: std::env::consts::ARCH.to_string(),
            virtualization,
            accel,
            ram_bytes,
            disk_free_bytes,
        }
    }
}

#[async_trait]
impl HostProbe for NativeHostProbe {
    async fn inspect(&self) -> Result<HostReport> {
        Ok(build_report(&self.gather()))
    }
}

/// `std::env::consts::OS` uses `"macos"`; keep that spelling (the report's `os` field matches
/// `report::accelerator_kind_for`).
fn os_family() -> &'static str {
    std::env::consts::OS
}

/// Total RAM and free bytes on `data_dir`'s volume, via `sysinfo`. `0` for anything it can't read.
fn mem_and_disk(data_dir: &Path) -> (u64, u64) {
    let ram = {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.total_memory()
    };

    let disks = sysinfo::Disks::new_with_refreshed_list();
    // The disk whose mount point is the longest prefix of `data_dir` is the one it lives on.
    let disk_free = disks
        .list()
        .iter()
        .filter(|d| data_dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map_or(0, sysinfo::Disk::available_space);

    (ram, disk_free)
}

// ---------------------------------------------------------------------------
// Per-OS accelerator + virtualization probes
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn platform_probe() -> (Virtualization, AccelSignal) {
    // `sysctl -n kern.hv_support` is `1` when Hypervisor.framework can be used — which on a Mac
    // also means CPU virtualization is on (Apple doesn't expose a firmware toggle).
    let hv = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.hv_support"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    match hv.as_deref() {
        Some("1") => (Virtualization::Enabled, AccelSignal::Ready),
        Some("0") => (Virtualization::Unknown, AccelSignal::Missing),
        _ => (Virtualization::Unknown, AccelSignal::Unknown),
    }
}

#[cfg(target_os = "linux")]
fn platform_probe() -> (Virtualization, AccelSignal) {
    use std::fs::OpenOptions;
    use std::io::ErrorKind;

    let dev_kvm = Path::new("/dev/kvm");
    if !dev_kvm.exists() {
        // No `/dev/kvm`: either the CPU can't virtualize, KVM isn't loaded, or it's a firmware
        // toggle. We can't cheaply tell those apart, so leave virtualization `Unknown` and let the
        // accelerator being `Missing` drive the verdict.
        return (Virtualization::Unknown, AccelSignal::Missing);
    }

    // `/dev/kvm` exists — try to open it read/write the way the emulator does.
    match OpenOptions::new().read(true).write(true).open(dev_kvm) {
        Ok(_) => (Virtualization::Enabled, AccelSignal::Ready),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => {
            (Virtualization::Enabled, AccelSignal::NoPermission)
        }
        Err(_) => (Virtualization::Enabled, AccelSignal::Unknown),
    }
}

#[cfg(target_os = "windows")]
fn platform_probe() -> (Virtualization, AccelSignal) {
    // Best-effort and UNTESTED on this dev machine (macOS). `powershell` reports whether the
    // Hypervisor Platform optional feature is enabled; virtualization-firmware state comes from
    // the CIM processor class.
    let feature = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform).State",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase());

    let virt_fw = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Processor).VirtualizationFirmwareEnabled",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase());

    let virtualization = match virt_fw.as_deref() {
        Some("true") => Virtualization::Enabled,
        Some("false") => Virtualization::DisabledInFirmware,
        _ => Virtualization::Unknown,
    };
    let accel = match feature.as_deref() {
        Some("enabled") => AccelSignal::Ready,
        Some("disabled") => AccelSignal::Missing,
        _ => AccelSignal::Unknown,
    };
    (virtualization, accel)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn platform_probe() -> (Virtualization, AccelSignal) {
    (Virtualization::Unknown, AccelSignal::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_returns_sane_os_and_arch() {
        let probe = NativeHostProbe::new(std::env::temp_dir());
        let s = probe.gather();
        assert!(["linux", "macos", "windows"].contains(&s.os.as_str()) || !s.os.is_empty());
        assert!(!s.arch.is_empty());
    }

    #[tokio::test]
    #[ignore = "real machine probe — run with `cargo test -p emu-host -- --ignored`"]
    async fn real_inspect_succeeds_on_this_host() {
        let probe = NativeHostProbe::new(std::env::temp_dir());
        let report = probe.inspect().await.expect("inspect ok");
        eprintln!(
            "host: os={} arch={} virt={:?} accel={:?}/{:?} ram={}B disk_free={}B verdict={:?}",
            report.os,
            report.arch,
            report.virtualization,
            report.accelerator.kind,
            report.accelerator.status,
            report.ram_bytes,
            report.disk_free_bytes,
            report.verdict
        );
        assert!(!report.os.is_empty());
        assert!(report.ram_bytes > 0);
    }
}
