//! [`build_report`] — pure derivation of a [`HostReport`] from [`HostSignals`].
//!
//! No I/O, no `#[cfg]`. Everything about "can this machine run an emulator, and if not, what fixes
//! it" lives here so it's exhaustively unit-testable with fabricated signals — including the M5
//! definition-of-done's "CI runner with no virtualization → `CannotRun`" case.

use emu_core::model::host::{
    Accelerator, AcceleratorKind, AcceleratorStatus, Fix, HostReport, Verdict, Virtualization,
};

use crate::signals::{AccelSignal, HostSignals};

/// Below this an Android emulator won't boot into anything usable.
const RAM_FLOOR_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Below this it boots but competes hard with everything else on the machine.
const RAM_LOW_BYTES: u64 = 8 * 1024 * 1024 * 1024;
/// A system image plus one AVD needs at least this much free.
const DISK_FLOOR_BYTES: u64 = 8 * 1024 * 1024 * 1024;
/// Below this you'll run out after a couple of system images.
const DISK_LOW_BYTES: u64 = 25 * 1024 * 1024 * 1024;

/// Turn raw signals into the full [`HostReport`] the UI renders.
#[must_use]
pub fn build_report(signals: &HostSignals) -> HostReport {
    let s = &with_emulator_verdict(signals);
    let kind = accelerator_kind_for(&s.os);
    let status = accelerator_status(s.accel);
    let fixes = fixes_for(kind, s);
    let verdict = verdict_for(kind, status, s);

    HostReport {
        os: s.os.clone(),
        arch: s.arch.clone(),
        virtualization: s.virtualization,
        accelerator: Accelerator { kind, status },
        disk_free_bytes: s.disk_free_bytes,
        ram_bytes: s.ram_bytes,
        verdict,
        fixes,
    }
}

/// Fold the emulator's own `-accel-check` into the OS-level guesses. A usable result clears a
/// false "virtualization is off" (on Windows with Hyper-V running, the firmware flag reads
/// `false` even though virtualization is on — the usual case on managed work PCs); an unusable
/// one means whatever the guesses said, acceleration is not available.
fn with_emulator_verdict(s: &HostSignals) -> HostSignals {
    let mut out = s.clone();
    match &s.emulator_check {
        Some(check) if check.usable => {
            out.virtualization = Virtualization::Enabled;
            out.accel = AccelSignal::Ready;
        }
        Some(_) if matches!(out.accel, AccelSignal::Ready | AccelSignal::Unknown) => {
            out.accel = AccelSignal::Missing;
        }
        _ => {}
    }
    out
}

/// The accelerator the Android emulator uses on this OS.
fn accelerator_kind_for(os: &str) -> AcceleratorKind {
    match os {
        "linux" => AcceleratorKind::Kvm,
        "macos" => AcceleratorKind::Hvf,
        // WHPX is the primary path on Windows; AEHD is the AMD-only fallback (a `Fix` offers it).
        "windows" => AcceleratorKind::Whpx,
        _ => AcceleratorKind::None,
    }
}

fn accelerator_status(sig: AccelSignal) -> AcceleratorStatus {
    match sig {
        AccelSignal::Ready => AcceleratorStatus::Ok,
        AccelSignal::NoPermission => AcceleratorStatus::NoPermission,
        AccelSignal::Missing => AcceleratorStatus::Missing,
        AccelSignal::Disabled => AcceleratorStatus::Disabled,
        AccelSignal::Unknown => AcceleratorStatus::Unknown,
    }
}

fn kind_label(kind: AcceleratorKind) -> &'static str {
    match kind {
        AcceleratorKind::Kvm => "KVM",
        AcceleratorKind::Whpx => "the Windows Hypervisor Platform",
        AcceleratorKind::Aehd => "AEHD",
        AcceleratorKind::Hvf => "Apple's Hypervisor",
        // `AcceleratorKind::None` and any future non_exhaustive variant.
        _ => "hardware acceleration",
    }
}

/// A rough `"12.0 GB"` for a user-facing message (decimal GB, matching how disks are sold).
/// Integer arithmetic — no float cast — so it stays exact and `-D warnings`-clean.
fn gb(bytes: u64) -> String {
    let whole = bytes / 1_000_000_000;
    let tenths = (bytes % 1_000_000_000) / 100_000_000;
    format!("{whole}.{tenths} GB")
}

fn verdict_for(kind: AcceleratorKind, status: AcceleratorStatus, s: &HostSignals) -> Verdict {
    // The emulator's own check is the only thing allowed to say "can't".
    if let Some(check) = s.emulator_check.as_ref().filter(|c| !c.usable) {
        return Verdict::CannotRun {
            reason: format!(
                "the Android emulator reports that {} can't be used on this computer ({}) — x86 \
                 emulators won't start without it",
                kind_label(kind),
                check.detail
            ),
        };
    }
    if s.ram_bytes > 0 && s.ram_bytes < RAM_FLOOR_BYTES {
        return Verdict::CannotRun {
            reason: format!(
                "only {} of RAM — an Android emulator needs at least 4 GB",
                gb(s.ram_bytes)
            ),
        };
    }
    if s.disk_free_bytes > 0 && s.disk_free_bytes < DISK_FLOOR_BYTES {
        return Verdict::CannotRun {
            reason: format!(
                "only {} free on the data volume — a system image plus one emulator needs about 8 GB",
                gb(s.disk_free_bytes)
            ),
        };
    }
    // Without an emulator to ask, these are guesses — warn, never block.
    if s.virtualization == Virtualization::DisabledInFirmware {
        return Verdict::Degraded {
            reason: "hardware virtualization looks turned off in your BIOS/UEFI — emulators may \
                     not start until it's on"
                .to_string(),
        };
    }
    if status == AcceleratorStatus::Missing {
        return Verdict::Degraded {
            reason: format!(
                "{} doesn't look available — emulators may not start or will be very slow",
                kind_label(kind)
            ),
        };
    }
    if status == AcceleratorStatus::NoPermission {
        return Verdict::Degraded {
            reason: format!("your user account can't use {} yet", kind_label(kind)),
        };
    }
    if status == AcceleratorStatus::Disabled {
        return Verdict::Degraded {
            reason: format!("{} is installed but switched off", kind_label(kind)),
        };
    }
    if s.ram_bytes > 0 && s.ram_bytes < RAM_LOW_BYTES {
        return Verdict::Degraded {
            reason: format!(
                "{} of RAM — emulators will be tight alongside your other apps",
                gb(s.ram_bytes)
            ),
        };
    }
    if s.disk_free_bytes > 0 && s.disk_free_bytes < DISK_LOW_BYTES {
        return Verdict::Degraded {
            reason: format!(
                "only {} free — you may run out after a couple of system images",
                gb(s.disk_free_bytes)
            ),
        };
    }
    Verdict::CanAccelerate
}

fn fixes_for(kind: AcceleratorKind, s: &HostSignals) -> Vec<Fix> {
    let mut fixes = Vec::new();

    let emulator_refused = s.emulator_check.as_ref().is_some_and(|c| !c.usable);
    if s.virtualization == Virtualization::DisabledInFirmware || emulator_refused {
        fixes.push(Fix {
            id: "enable-virtualization".to_string(),
            title: "Enable virtualization in your BIOS/UEFI".to_string(),
            scriptable: false,
            needs_reboot: true,
            description: "Reboot into firmware setup and turn on Intel VT-x / AMD-V (sometimes \
                          labelled \"SVM Mode\" or \"Virtualization Technology\"). Emulator Studio \
                          can't change a firmware setting for you. On a work-managed computer this \
                          is often locked by policy — ask your IT team to allow virtualization \
                          (and the Windows Hypervisor Platform on Windows)."
                .to_string(),
        });
    }

    match (kind, s.accel) {
        (AcceleratorKind::Kvm, AccelSignal::NoPermission) => fixes.push(Fix {
            id: "add-kvm-group".to_string(),
            title: "Add your user to the kvm group".to_string(),
            scriptable: true,
            needs_reboot: false,
            description: "Runs `sudo usermod -aG kvm $USER`. Log out and back in (or reboot) for \
                          it to take effect."
                .to_string(),
        }),
        (AcceleratorKind::Kvm, AccelSignal::Missing) => fixes.push(Fix {
            id: "install-kvm".to_string(),
            title: "Install KVM".to_string(),
            scriptable: false,
            needs_reboot: false,
            description:
                "Install your distro's KVM packages (e.g. `qemu-kvm libvirt-daemon-system` \
                          on Debian/Ubuntu) and make sure the `kvm` kernel module is loaded."
                    .to_string(),
        }),
        (AcceleratorKind::Whpx, AccelSignal::Missing | AccelSignal::Disabled) => fixes.push(Fix {
            id: "enable-whpx".to_string(),
            title: "Enable the Windows Hypervisor Platform".to_string(),
            scriptable: true,
            needs_reboot: true,
            description: "Runs `dism /online /enable-feature /featurename:HypervisorPlatform /all \
                          /norestart`, then a reboot is required."
                .to_string(),
        }),
        (AcceleratorKind::Whpx, AccelSignal::NoPermission) => fixes.push(Fix {
            id: "enable-aehd".to_string(),
            title: "Install AEHD (AMD hosts)".to_string(),
            scriptable: true,
            needs_reboot: true,
            description: "On an AMD CPU without WHPX, install the Android Emulator Hypervisor \
                          Driver (AEHD) — the HAXM successor — and reboot."
                .to_string(),
        }),
        _ => {}
    }

    fixes
}

#[cfg(test)]
mod tests {
    use emu_core::model::host::AccelCheck;

    use super::*;

    fn healthy_mac() -> HostSignals {
        HostSignals {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            virtualization: Virtualization::Enabled,
            accel: AccelSignal::Ready,
            ram_bytes: 32 * 1024 * 1024 * 1024,
            disk_free_bytes: 400 * 1_000_000_000,
            emulator_check: None,
        }
    }

    #[test]
    fn healthy_mac_can_accelerate_with_no_fixes() {
        let r = build_report(&healthy_mac());
        assert_eq!(r.verdict, Verdict::CanAccelerate);
        assert_eq!(r.accelerator.kind, AcceleratorKind::Hvf);
        assert_eq!(r.accelerator.status, AcceleratorStatus::Ok);
        assert!(r.fixes.is_empty());
    }

    #[test]
    fn firmware_flag_alone_warns_and_offers_the_bios_fix_but_never_blocks() {
        let s = HostSignals {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            virtualization: Virtualization::DisabledInFirmware,
            accel: AccelSignal::Missing,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            disk_free_bytes: 100 * 1_000_000_000,
            emulator_check: None,
        };
        let r = build_report(&s);
        // A guess (no emulator to ask yet) must not block anyone.
        assert!(matches!(r.verdict, Verdict::Degraded { .. }));
        if let Verdict::Degraded { reason } = &r.verdict {
            assert!(reason.contains("BIOS"));
        }
        assert!(r
            .fixes
            .iter()
            .any(|f| f.id == "enable-virtualization" && !f.scriptable));
    }

    #[test]
    fn kvm_no_permission_is_degraded_with_add_kvm_group() {
        let s = HostSignals {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            virtualization: Virtualization::Enabled,
            accel: AccelSignal::NoPermission,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            disk_free_bytes: 100 * 1_000_000_000,
            emulator_check: None,
        };
        let r = build_report(&s);
        assert!(matches!(r.verdict, Verdict::Degraded { .. }));
        let fix = r
            .fixes
            .iter()
            .find(|f| f.id == "add-kvm-group")
            .expect("fix present");
        assert!(fix.scriptable);
        assert!(!fix.needs_reboot);
    }

    #[test]
    fn whpx_missing_is_degraded_with_a_reboot_fix() {
        let s = HostSignals {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            virtualization: Virtualization::Enabled,
            accel: AccelSignal::Missing,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            disk_free_bytes: 100 * 1_000_000_000,
            emulator_check: None,
        };
        let r = build_report(&s);
        assert!(matches!(r.verdict, Verdict::Degraded { .. }));
        let fix = r
            .fixes
            .iter()
            .find(|f| f.id == "enable-whpx")
            .expect("fix present");
        assert!(fix.scriptable && fix.needs_reboot);
    }

    #[test]
    fn low_ram_is_degraded() {
        let mut s = healthy_mac();
        s.ram_bytes = 6 * 1024 * 1024 * 1024;
        assert!(matches!(build_report(&s).verdict, Verdict::Degraded { .. }));
    }

    #[test]
    fn too_little_ram_cannot_run() {
        let mut s = healthy_mac();
        s.ram_bytes = 2 * 1024 * 1024 * 1024;
        assert!(matches!(
            build_report(&s).verdict,
            Verdict::CannotRun { .. }
        ));
    }

    #[test]
    fn too_little_disk_cannot_run() {
        let mut s = healthy_mac();
        s.disk_free_bytes = 3 * 1_000_000_000;
        let r = build_report(&s);
        assert!(matches!(r.verdict, Verdict::CannotRun { .. }));
    }

    #[test]
    fn unknown_signals_do_not_hard_block() {
        let s = HostSignals {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            virtualization: Virtualization::Unknown,
            accel: AccelSignal::Unknown,
            ram_bytes: 0,
            disk_free_bytes: 0,
            emulator_check: None,
        };
        // Nothing is known to be wrong → not a CannotRun.
        assert_eq!(build_report(&s).verdict, Verdict::CanAccelerate);
    }

    fn windows_with_hyper_v() -> HostSignals {
        // `Win32_Processor.VirtualizationFirmwareEnabled` reads false while Hyper-V is running.
        HostSignals {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            virtualization: Virtualization::DisabledInFirmware,
            accel: AccelSignal::Unknown,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            disk_free_bytes: 100 * 1_000_000_000,
            emulator_check: None,
        }
    }

    #[test]
    fn a_usable_emulator_check_clears_a_false_firmware_warning() {
        let mut s = windows_with_hyper_v();
        s.emulator_check = Some(AccelCheck {
            usable: true,
            detail: "WHPX is installed and usable.".to_string(),
        });
        let r = build_report(&s);
        assert_eq!(r.verdict, Verdict::CanAccelerate);
        assert_eq!(r.virtualization, Virtualization::Enabled);
        assert!(r.fixes.is_empty());
    }

    #[test]
    fn an_unusable_emulator_check_is_the_only_hard_block() {
        let mut s = windows_with_hyper_v();
        s.emulator_check = Some(AccelCheck {
            usable: false,
            detail: "WHPX is not installed".to_string(),
        });
        let r = build_report(&s);
        let Verdict::CannotRun { reason } = &r.verdict else {
            panic!("expected CannotRun, got {:?}", r.verdict);
        };
        assert!(reason.contains("WHPX is not installed"));
        assert!(r.fixes.iter().any(|f| f.id == "enable-virtualization"));
        assert!(r.fixes.iter().any(|f| f.id == "enable-whpx"));
    }
}
