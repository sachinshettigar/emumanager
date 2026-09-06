//! Per-OS subcommand bodies. Off its platform, every mutating command is `notApplicable` (exit 0).
//!
//! `#[cfg]` picks the real implementation; the `not_applicable` helper covers the other OSes so
//! the app can call any command on any host and get a clean structured answer.

use std::process::Command as Proc;

use crate::{Command, Outcome};

fn not_applicable(cmd: Command, what: &str) -> Outcome {
    Outcome::new(
        cmd,
        "notApplicable",
        format!(
            "{what} — nothing for this helper to do on {}.",
            std::env::consts::OS
        ),
    )
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn from_proc(
    cmd: Command,
    mut proc: Proc,
    ok_reboot: bool,
    ok_msg: &str,
    fail_prefix: &str,
) -> Outcome {
    match proc.output() {
        Ok(out) if out.status.success() => Outcome::new(
            cmd,
            if ok_reboot { "needsReboot" } else { "ok" },
            ok_msg.to_string(),
        ),
        Ok(out) => Outcome::new(
            cmd,
            "failed",
            format!(
                "{fail_prefix}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ),
        Err(e) => Outcome::new(
            cmd,
            "failed",
            format!("{fail_prefix}: could not run the command ({e})"),
        ),
    }
}

// ---------------------------------------------------------------------------
// macOS
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
pub fn check(cmd: Command) -> Outcome {
    let hv = Proc::new("/usr/sbin/sysctl")
        .args(["-n", "kern.hv_support"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    match hv.as_deref() {
        Some("1") => Outcome::new(
            cmd,
            "ok",
            "Apple's Hypervisor (HVF) is available — no setup needed.",
        ),
        _ => Outcome::new(
            cmd,
            "failed",
            "kern.hv_support is not 1 — this Mac can't use hardware acceleration.",
        ),
    }
}

#[cfg(target_os = "macos")]
pub fn enable_whpx(cmd: Command) -> Outcome {
    not_applicable(cmd, "The Windows Hypervisor Platform is a Windows feature")
}
#[cfg(target_os = "macos")]
pub fn enable_aehd(cmd: Command) -> Outcome {
    not_applicable(cmd, "AEHD is a Windows driver")
}
#[cfg(target_os = "macos")]
pub fn add_kvm_group(cmd: Command) -> Outcome {
    not_applicable(cmd, "The kvm group is a Linux concept")
}

// ---------------------------------------------------------------------------
// Linux
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
pub fn check(cmd: Command) -> Outcome {
    use std::fs::OpenOptions;
    match OpenOptions::new().read(true).write(true).open("/dev/kvm") {
        Ok(_) => Outcome::new(cmd, "ok", "/dev/kvm is present and usable."),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Outcome::new(
            cmd,
            "ok",
            "/dev/kvm exists but this user can't open it — run `add-kvm-group`.",
        ),
        Err(_) => Outcome::new(cmd, "failed", "/dev/kvm is missing — install KVM."),
    }
}

#[cfg(target_os = "linux")]
pub fn add_kvm_group(cmd: Command) -> Outcome {
    // Runs as root, so `$USER` would be `root`. The invoking user is `$SUDO_USER` (sudo) or
    // resolvable from `$PKEXEC_UID` (pkexec).
    let user = std::env::var("SUDO_USER").ok().or_else(|| {
        std::env::var("PKEXEC_UID").ok().and_then(|uid| {
            Proc::new("id")
                .args(["-nu", &uid])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
    });
    let Some(user) = user.filter(|u| !u.is_empty() && u != "root") else {
        return Outcome::new(
            cmd,
            "failed",
            "couldn't tell which user to add to the kvm group ($SUDO_USER is unset).",
        );
    };
    from_proc(
        cmd,
        {
            let mut p = Proc::new("usermod");
            p.args(["-aG", "kvm", &user]);
            p
        },
        false,
        &format!("Added {user} to the kvm group. Log out and back in for it to take effect."),
        "usermod failed",
    )
}

#[cfg(target_os = "linux")]
pub fn enable_whpx(cmd: Command) -> Outcome {
    not_applicable(cmd, "The Windows Hypervisor Platform is a Windows feature")
}
#[cfg(target_os = "linux")]
pub fn enable_aehd(cmd: Command) -> Outcome {
    not_applicable(cmd, "AEHD is a Windows driver")
}

// ---------------------------------------------------------------------------
// Windows  (best-effort — NOT run on a real Windows host yet; verify in M6)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn check(cmd: Command) -> Outcome {
    let state = Proc::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform).State",
        ])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase());
    match state.as_deref() {
        Some("enabled") => Outcome::new(cmd, "ok", "The Windows Hypervisor Platform is enabled."),
        Some("disabled") => Outcome::new(
            cmd,
            "ok",
            "The Windows Hypervisor Platform is disabled — run `enable-whpx`.",
        ),
        _ => Outcome::new(
            cmd,
            "failed",
            "couldn't query the Hypervisor Platform feature state.",
        ),
    }
}

#[cfg(target_os = "windows")]
pub fn enable_whpx(cmd: Command) -> Outcome {
    let mut p = Proc::new("dism");
    p.args([
        "/online",
        "/enable-feature",
        "/featurename:HypervisorPlatform",
        "/all",
        "/norestart",
    ]);
    from_proc(
        cmd,
        p,
        true,
        "Enabled the Windows Hypervisor Platform. Reboot to finish.",
        "dism failed",
    )
}

#[cfg(target_os = "windows")]
pub fn enable_aehd(cmd: Command) -> Outcome {
    Outcome::new(
        cmd,
        "failed",
        "AEHD can't be scripted yet — install it from the Android SDK's `extras` \
         (`extras;google;Android_Emulator_Hypervisor_Driver`) and run its `silent_install.bat` as \
         Administrator.",
    )
}

#[cfg(target_os = "windows")]
pub fn add_kvm_group(cmd: Command) -> Outcome {
    not_applicable(cmd, "The kvm group is a Linux concept")
}

// ---------------------------------------------------------------------------
// Anything else
// ---------------------------------------------------------------------------

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn check(cmd: Command) -> Outcome {
    not_applicable(cmd, "Unsupported OS")
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn enable_whpx(cmd: Command) -> Outcome {
    not_applicable(cmd, "Unsupported OS")
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn enable_aehd(cmd: Command) -> Outcome {
    not_applicable(cmd, "Unsupported OS")
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn add_kvm_group(cmd: Command) -> Outcome {
    not_applicable(cmd, "Unsupported OS")
}
