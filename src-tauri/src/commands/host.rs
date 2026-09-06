//! Host commands (milestone M5, task 0026): `probe_host` (run `emu-host`, snapshot the report)
//! and `run_helper` (invoke `emu-helper` with OS elevation, parse its JSON).
//!
//! All of the verdict/fixes logic lives in `emu_host::build_report` — this file only maps the
//! `HostReport` (whose `u64` fields specta won't export, and whose enums aren't `specta::Type`)
//! onto flat DTOs, and wraps the elevated `emu-helper` call.

use std::path::{Path, PathBuf};

use emu_core::model::host::{
    AcceleratorKind, AcceleratorStatus, HostReport, Verdict, Virtualization,
};
use emu_core::ports::HostProbe as _;
use emu_core::registry::HostSnapshotRow;
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use time::OffsetDateTime;

use crate::ipc_error::IpcError;
use crate::provider_state::ManagedProvider;

// ---------------------------------------------------------------------------
// probe_host
// ---------------------------------------------------------------------------

/// One remediation from the host report.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HostFixDto {
    pub id: String,
    pub title: String,
    /// `true` when `run_helper(id)` can perform it (with elevation).
    pub scriptable: bool,
    pub needs_reboot: bool,
    pub description: String,
}

/// The host-readiness picture for the Dependencies screen.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HostReportDto {
    pub os: String,
    pub arch: String,
    /// `enabled` / `disabledInFirmware` / `unknown`.
    pub virtualization: String,
    /// `kvm` / `whpx` / `aehd` / `hvf` / `none`.
    pub accelerator_kind: String,
    /// `ok` / `missing` / `noPermission` / `disabled` / `unknown`.
    pub accelerator_status: String,
    /// Free space on the data volume, in MB (`u32` — specta won't export a bigint-risking `u64`).
    pub disk_free_mb: u32,
    /// Total RAM in MB.
    pub ram_mb: u32,
    /// `canAccelerate` / `degraded` / `cannotRun`.
    pub verdict: String,
    /// The `degraded` / `cannotRun` reason; empty for `canAccelerate`.
    pub verdict_reason: String,
    pub fixes: Vec<HostFixDto>,
}

fn virt_str(v: Virtualization) -> &'static str {
    match v {
        Virtualization::Enabled => "enabled",
        Virtualization::DisabledInFirmware => "disabledInFirmware",
        _ => "unknown",
    }
}

fn accel_kind_str(k: AcceleratorKind) -> &'static str {
    match k {
        AcceleratorKind::Kvm => "kvm",
        AcceleratorKind::Whpx => "whpx",
        AcceleratorKind::Aehd => "aehd",
        AcceleratorKind::Hvf => "hvf",
        _ => "none",
    }
}

fn accel_status_str(s: AcceleratorStatus) -> &'static str {
    match s {
        AcceleratorStatus::Ok => "ok",
        AcceleratorStatus::Missing => "missing",
        AcceleratorStatus::NoPermission => "noPermission",
        AcceleratorStatus::Disabled => "disabled",
        _ => "unknown",
    }
}

fn mb(bytes: u64) -> u32 {
    u32::try_from(bytes / (1024 * 1024)).unwrap_or(u32::MAX)
}

impl From<&HostReport> for HostReportDto {
    fn from(r: &HostReport) -> Self {
        let (verdict, verdict_reason) = match &r.verdict {
            Verdict::CanAccelerate => ("canAccelerate".to_string(), String::new()),
            Verdict::Degraded { reason } => ("degraded".to_string(), reason.clone()),
            Verdict::CannotRun { reason } => ("cannotRun".to_string(), reason.clone()),
            _ => ("cannotRun".to_string(), "unknown host state".to_string()),
        };
        Self {
            os: r.os.clone(),
            arch: r.arch.clone(),
            virtualization: virt_str(r.virtualization).to_string(),
            accelerator_kind: accel_kind_str(r.accelerator.kind).to_string(),
            accelerator_status: accel_status_str(r.accelerator.status).to_string(),
            disk_free_mb: mb(r.disk_free_bytes),
            ram_mb: mb(r.ram_bytes),
            verdict,
            verdict_reason,
            fixes: r
                .fixes
                .iter()
                .map(|f| HostFixDto {
                    id: f.id.clone(),
                    title: f.title.clone(),
                    scriptable: f.scriptable,
                    needs_reboot: f.needs_reboot,
                    description: f.description.clone(),
                })
                .collect(),
        }
    }
}

/// Probe the machine for emulator readiness. Also records a `host_snapshots` row (best-effort).
#[tauri::command]
#[specta::specta]
pub async fn probe_host(
    app: AppHandle,
    mgr: State<'_, ManagedProvider>,
) -> Result<HostReportDto, IpcError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| IpcError::new("fs_error", format!("resolving the data directory: {e}")))?;

    let report = emu_host::NativeHostProbe::new(data_dir)
        .inspect()
        .await
        .map_err(IpcError::from)?;

    // Snapshot it — never fatal to the probe itself.
    if let Ok(provider) = mgr.get().await {
        if let Ok(json) = serde_json::to_string(&report) {
            let row = HostSnapshotRow {
                id: emu_core::model::emulator::EmulatorId::generate().0,
                captured_at: OffsetDateTime::now_utc(),
                report_json: json,
            };
            let _ = provider.registry().insert_host_snapshot(&row).await;
        }
    }

    Ok(HostReportDto::from(&report))
}

// ---------------------------------------------------------------------------
// run_helper
// ---------------------------------------------------------------------------

/// The parsed result of one elevated `emu-helper` run.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HelperOutcomeDto {
    pub command: String,
    /// `ok` / `failed` / `notApplicable` / `needsReboot`.
    pub status: String,
    pub message: String,
    pub needs_reboot: bool,
}

/// The fix ids `run_helper` can actually script (the rest are manual — see `emu_host`).
const SCRIPTABLE: &[(&str, &str)] = &[
    ("enable-whpx", "enable-whpx"),
    ("enable-aehd", "enable-aehd"),
    ("add-kvm-group", "add-kvm-group"),
];

/// Locate the `emu-helper` binary: next to the running app (a bundled build), else the sibling
/// `emu-helper[.exe]` in the same `target/<profile>` dir (dev).
fn helper_path() -> Result<PathBuf, IpcError> {
    let exe = std::env::current_exe()
        .map_err(|e| IpcError::new("fs_error", format!("locating the app binary: {e}")))?;
    let dir = exe.parent().unwrap_or(Path::new("."));
    let name = if cfg!(windows) {
        "emu-helper.exe"
    } else {
        "emu-helper"
    };
    let candidate = dir.join(name);
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(IpcError::new(
        "not_found",
        "couldn't find the emu-helper binary next to the app",
    ))
}

/// Build the `(program, args)` that runs `helper <sub>` **with an elevation prompt**, per OS. Pure
/// so it's unit-tested without actually elevating.
fn elevated_argv(helper: &Path, sub: &str, os: &str) -> (String, Vec<String>) {
    let h = helper.display().to_string();
    match os {
        "macos" => (
            "osascript".to_string(),
            vec![
                "-e".to_string(),
                format!("do shell script \"'{h}' {sub}\" with administrator privileges"),
            ],
        ),
        "linux" => ("pkexec".to_string(), vec![h, sub.to_string()]),
        "windows" => (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!("Start-Process -FilePath '{h}' -ArgumentList '{sub}' -Verb RunAs -Wait"),
            ],
        ),
        _ => (h, vec![sub.to_string()]),
    }
}

/// Run a scriptable host fix with OS elevation and return `emu-helper`'s structured outcome.
#[tauri::command]
#[specta::specta]
pub async fn run_helper(fix_id: String) -> Result<HelperOutcomeDto, IpcError> {
    let Some(&(_, sub)) = SCRIPTABLE.iter().find(|(id, _)| *id == fix_id) else {
        return Err(IpcError::new(
            "invalid",
            format!("'{fix_id}' has to be done manually — see the fix description"),
        ));
    };
    let helper = helper_path()?;
    let os = std::env::consts::OS;
    let (program, args) = elevated_argv(&helper, sub, os);

    let output = tokio::process::Command::new(&program)
        .args(&args)
        .output()
        .await
        .map_err(|e| IpcError::new("process_failed", format!("running {program}: {e}")))?;

    // A cancelled elevation prompt: osascript says "User canceled" (-128); pkexec exits 126.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        && (stderr.contains("User canceled")
            || stderr.contains("cancelled")
            || output.status.code() == Some(126))
    {
        return Err(IpcError::new(
            "cancelled",
            "the administrator prompt was dismissed",
        ));
    }

    // Windows `Start-Process` doesn't forward the child's stdout — synthesize an outcome and let
    // the caller re-probe. macOS/Linux forward `emu-helper`'s JSON line.
    let stdout = String::from_utf8_lossy(&output.stdout);
    if os == "windows" {
        return Ok(HelperOutcomeDto {
            command: sub.to_string(),
            status: if output.status.success() {
                "ok"
            } else {
                "failed"
            }
            .to_string(),
            message: "Ran the fix with elevation. Re-probe the host to see the new state."
                .to_string(),
            needs_reboot: sub == "enable-whpx" || sub == "enable-aehd",
        });
    }

    let line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| {
            IpcError::new(
                "process_failed",
                format!("emu-helper produced no JSON (stderr: {})", stderr.trim()),
            )
        })?;
    let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
        IpcError::new(
            "process_failed",
            format!("emu-helper output wasn't JSON: {e}"),
        )
    })?;
    Ok(HelperOutcomeDto {
        command: v["command"].as_str().unwrap_or(sub).to_string(),
        status: v["status"].as_str().unwrap_or("failed").to_string(),
        message: v["message"].as_str().unwrap_or_default().to_string(),
        needs_reboot: v["needsReboot"].as_bool().unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevated_argv_wraps_per_os() {
        let h = Path::new("/opt/app/emu-helper");
        let (p, a) = elevated_argv(h, "add-kvm-group", "macos");
        assert_eq!(p, "osascript");
        assert!(a.last().unwrap().contains("with administrator privileges"));
        assert!(a.last().unwrap().contains("add-kvm-group"));

        let (p, a) = elevated_argv(h, "enable-whpx", "linux");
        assert_eq!(p, "pkexec");
        assert_eq!(a, ["/opt/app/emu-helper", "enable-whpx"]);

        let (p, a) = elevated_argv(h, "enable-whpx", "windows");
        assert_eq!(p, "powershell");
        assert!(a.iter().any(|s| s.contains("RunAs")));
    }

    #[test]
    fn run_helper_rejects_a_non_scriptable_fix() {
        let out = tokio_test_block_on(run_helper("enable-virtualization".to_string()));
        let err = out.unwrap_err();
        assert_eq!(err.code, "invalid");
    }

    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn host_report_dto_maps_verdict_and_units() {
        use emu_core::model::host::{Accelerator, Fix};
        let report = HostReport {
            os: "linux".into(),
            arch: "x86_64".into(),
            virtualization: Virtualization::Enabled,
            accelerator: Accelerator {
                kind: AcceleratorKind::Kvm,
                status: AcceleratorStatus::NoPermission,
            },
            disk_free_bytes: 50 * 1024 * 1024 * 1024,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            verdict: Verdict::Degraded {
                reason: "not in the kvm group".into(),
            },
            fixes: vec![Fix {
                id: "add-kvm-group".into(),
                title: "Add your user to the kvm group".into(),
                scriptable: true,
                needs_reboot: false,
                description: "usermod -aG kvm".into(),
            }],
        };
        let dto = HostReportDto::from(&report);
        assert_eq!(dto.verdict, "degraded");
        assert_eq!(dto.verdict_reason, "not in the kvm group");
        assert_eq!(dto.accelerator_status, "noPermission");
        assert_eq!(dto.ram_mb, 16 * 1024);
        assert_eq!(dto.fixes.len(), 1);
        assert!(dto.fixes[0].scriptable);
    }
}
