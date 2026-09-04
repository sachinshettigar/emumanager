//! [`HostReport`] — the result of probing the machine for emulator readiness.
//!
//! Produced by the `HostProbe` port (implemented in `emu-host`, milestone M5).

use serde::{Deserialize, Serialize};

/// Whether hardware virtualization is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Virtualization {
    /// CPU virtualization present and enabled.
    Enabled,
    /// Supported by the CPU but switched off in firmware/BIOS.
    DisabledInFirmware,
    /// Could not be determined.
    Unknown,
}

/// Kind of hypervisor / acceleration layer the emulator would use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum AcceleratorKind {
    /// Linux KVM.
    Kvm,
    /// Windows Hypervisor Platform.
    Whpx,
    /// Android Emulator Hypervisor Driver (AMD, replaces HAXM).
    Aehd,
    /// Apple Hypervisor.framework.
    Hvf,
    /// No accelerator — software only.
    None,
}

/// Status of the chosen [`AcceleratorKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum AcceleratorStatus {
    /// Ready to use.
    Ok,
    /// Not installed / driver missing.
    Missing,
    /// Present but the current user lacks permission (e.g. not in the `kvm` group).
    NoPermission,
    /// Present but disabled (service stopped, feature off).
    Disabled,
    /// State could not be determined.
    Unknown,
}

/// The accelerator picture for the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accelerator {
    /// Which accelerator applies on this OS/CPU.
    pub kind: AcceleratorKind,
    /// Its current status.
    pub status: AcceleratorStatus,
}

/// Overall go/no-go verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
#[non_exhaustive]
pub enum Verdict {
    /// Emulators will run with hardware acceleration.
    CanAccelerate,
    /// Emulators will run, but slowly / with caveats.
    Degraded {
        /// Why it is degraded.
        reason: String,
    },
    /// Emulators cannot run until something is fixed.
    CannotRun {
        /// What is blocking.
        reason: String,
    },
}

/// A remediation the app can offer (and sometimes apply via `emu-helper`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    /// Stable id, e.g. `enable-whpx`.
    pub id: String,
    /// Short imperative title.
    pub title: String,
    /// Whether `emu-helper` can perform this automatically (with elevation).
    pub scriptable: bool,
    /// Whether applying it requires a reboot.
    pub needs_reboot: bool,
    /// Full explanation, including any manual steps.
    pub description: String,
}

/// Snapshot of host capabilities relevant to running emulators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostReport {
    /// OS family, e.g. `"linux"`, `"windows"`, `"macos"`.
    pub os: String,
    /// CPU architecture, e.g. `"x86_64"`, `"aarch64"`.
    pub arch: String,
    /// Virtualization availability.
    pub virtualization: Virtualization,
    /// Accelerator kind + status.
    pub accelerator: Accelerator,
    /// Free disk space in the data directory's volume, in bytes.
    pub disk_free_bytes: u64,
    /// Total system RAM in bytes.
    pub ram_bytes: u64,
    /// Overall verdict.
    pub verdict: Verdict,
    /// Ordered list of applicable fixes (most impactful first).
    pub fixes: Vec<Fix>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_is_tagged_and_camel_cased() {
        let v = Verdict::CannotRun {
            reason: "no virtualization".into(),
        };
        let json = serde_json::to_value(&v).expect("ser");
        assert_eq!(json["kind"], "cannotRun");
        assert_eq!(json["reason"], "no virtualization");
        let back: Verdict = serde_json::from_value(json).expect("de");
        assert_eq!(v, back);
    }

    #[test]
    fn host_report_round_trips() {
        let r = HostReport {
            os: "linux".into(),
            arch: "x86_64".into(),
            virtualization: Virtualization::Enabled,
            accelerator: Accelerator {
                kind: AcceleratorKind::Kvm,
                status: AcceleratorStatus::NoPermission,
            },
            disk_free_bytes: 200_000_000_000,
            ram_bytes: 32_000_000_000,
            verdict: Verdict::Degraded {
                reason: "user not in kvm group".into(),
            },
            fixes: vec![Fix {
                id: "add-kvm-group".into(),
                title: "Add your user to the kvm group".into(),
                scriptable: true,
                needs_reboot: false,
                description: "Runs `usermod -aG kvm $USER`; log out and back in.".into(),
            }],
        };
        let json = serde_json::to_string(&r).expect("ser");
        let back: HostReport = serde_json::from_str(&json).expect("de");
        assert_eq!(r, back);
    }
}
