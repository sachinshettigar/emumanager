//! Emulator commands (milestone M2, task 0017): the device + system-image catalogs for the
//! Create wizard, `create`/`launch`/`stop` driving the real [`emu_android::provider::AndroidProvider`],
//! and the Dashboard's live emulator list.
//!
//! Same shape as `commands::toolchain`: thin adapters that fetch/parse in the shell (device XML
//! out of the installed `sdklib` jar, `sys-img2-3.xml` manifests over HTTP — exactly as
//! `toolchain::resolve_catalog` does for the component catalog), construct the real port impls,
//! and map results into `#[derive(specta::Type)]` DTOs or [`IpcError`].
//!
//! `AndroidProvider` is built **per command**, not held as shared state. Its in-memory
//! spawned-child map (task 0016) therefore doesn't persist across calls, so `stop` relies on the
//! graceful `adb emu kill` path rather than a held handle. That's fine for M2's "create → boot →
//! stop" flow; a shared, managed provider (so `stop` can force-kill and the app can reap children
//! on exit) is part of M3's "Registry & reliable tracking" milestone.

use std::path::Path;
use std::sync::Arc;

use emu_android::provider::{AndroidProvider, TrackedEmulator};
use emu_core::model::device::{DeviceProfile, FormFactor};
use emu_core::model::emulator::{EmulatorId, Hardware, RunState};
use emu_core::model::image::ImageCoord;
use emu_core::model::job::{JobHandle, JobId, Progress};
use emu_core::model::plan::CreateSpec;
use emu_core::model::HostOs;
use emu_core::provider::{LaunchOpts, Provider as _};
use emu_core::registry::Registry;
use emu_core::CoreError;
use futures_util::future::try_join_all;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::ipc_error::IpcError;
use crate::ports::{NativeFs, NativeProcessRunner};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn require_host_os() -> Result<HostOs, IpcError> {
    HostOs::current().ok_or_else(|| {
        IpcError::new(
            "unsupported",
            "this operating system isn't supported by the Android SDK",
        )
    })
}

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, IpcError> {
    app.path()
        .app_data_dir()
        .map_err(|e| IpcError::new("fs_error", format!("resolving the app data directory: {e}")))
}

/// Build an [`AndroidProvider`] over the real ports for this call.
async fn provider(app: &AppHandle) -> Result<AndroidProvider, IpcError> {
    let os = require_host_os()?;
    let data_dir = app_data_dir(app)?;
    let registry = Registry::open(&data_dir).await.map_err(IpcError::from)?;
    Ok(AndroidProvider::new(
        Arc::new(NativeProcessRunner),
        Arc::new(NativeFs),
        registry,
        data_dir,
        os,
    ))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, IpcError> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| IpcError::new("download_failed", format!("fetching {url}: {e}")))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| IpcError::new("download_failed", format!("reading {url}: {e}")))?;
    Ok(bytes.to_vec())
}

// ---------------------------------------------------------------------------
// Device catalog
// ---------------------------------------------------------------------------

/// A hardware profile for the Create wizard's device step.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// `avdmanager -d` id, e.g. `pixel_6`.
    pub id: String,
    /// Human name, e.g. `Pixel 6`.
    pub name: String,
    /// Manufacturer label; empty when unknown.
    pub oem: String,
    /// `phone` / `tablet` / `foldable` / `wear` / `tv` / `automotive` / `desktop`.
    pub form_factor: String,
    /// Manufacturer-recommended RAM in MiB.
    pub ram_mb: u32,
    /// `"<width> × <height>"` in pixels.
    pub resolution: String,
    /// Screen density in dpi.
    pub density_dpi: u32,
    /// Physical diagonal in inches.
    pub diagonal_in: f32,
}

fn form_factor_label(form_factor: FormFactor) -> &'static str {
    match form_factor {
        FormFactor::Phone => "phone",
        FormFactor::Tablet => "tablet",
        FormFactor::Foldable => "foldable",
        FormFactor::Wear => "wear",
        FormFactor::Tv => "tv",
        FormFactor::Automotive => "automotive",
        FormFactor::Desktop => "desktop",
        _ => "other",
    }
}

impl From<&DeviceProfile> for DeviceInfo {
    fn from(device: &DeviceProfile) -> Self {
        Self {
            id: device.id.clone(),
            name: device.display_name.clone(),
            oem: device.oem.clone(),
            form_factor: form_factor_label(device.form_factor).to_string(),
            ram_mb: device.default_ram_mb,
            resolution: format!("{} × {}", device.screen.width_px, device.screen.height_px),
            density_dpi: device.screen.density_dpi,
            diagonal_in: device.screen.diagonal_in,
        }
    }
}

/// The device XML lives inside `cmdline-tools`' `sdklib` jar — the exact relative path has
/// wobbled across `cmdline-tools` revisions, so try the ones seen in the wild
/// (`crates/emu-android/src/devices.rs` module doc), first hit wins.
fn read_sdklib_jar(sdk_root: &Path) -> Result<Vec<u8>, IpcError> {
    const CANDIDATES: &[&str] = &[
        "cmdline-tools/latest/lib/sdklib/sdklib.core.jar",
        "cmdline-tools/latest/lib/sdklib/tools.sdklib.jar",
        "cmdline-tools/latest/lib/sdklib.jar",
    ];
    for relative in CANDIDATES {
        if let Ok(bytes) = std::fs::read(sdk_root.join(relative)) {
            return Ok(bytes);
        }
    }
    Err(IpcError::new(
        "not_found",
        "couldn't find the device-catalog jar under the installed cmdline-tools — install the \
         SDK on the Dependencies screen first",
    ))
}

/// Every Google device profile, parsed from the installed `sdklib` jar.
#[tauri::command]
#[specta::specta]
pub async fn list_devices(app: AppHandle) -> Result<Vec<DeviceInfo>, IpcError> {
    let provider = provider(&app).await?;
    let sdk_root = provider.sdk_root().await.map_err(IpcError::from)?;
    let jar = read_sdklib_jar(&sdk_root)?;
    let mut devices: Vec<DeviceInfo> = emu_android::devices::parse_from_jar(&jar)
        .map_err(IpcError::from)?
        .iter()
        .map(DeviceInfo::from)
        .collect();
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(devices)
}

// ---------------------------------------------------------------------------
// System-image catalog
// ---------------------------------------------------------------------------

/// A system image for the Create wizard's image step.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    /// `sdkmanager` package path — the id passed back to `create_emulator`.
    pub coord: String,
    /// Android API level.
    pub api: u32,
    /// Marketing Android version, e.g. `"14"`.
    pub android_version: String,
    /// `default` / `google_apis` / `google_apis_playstore` / …
    pub image_type: String,
    /// `x86_64` / `arm64-v8a` / …
    pub abi: String,
    /// Package revision.
    pub revision: String,
    /// Archive size in bytes — `u32` (specta won't export a bigint-risking `u64`; every real
    /// image is well under 4 GiB), saturating.
    pub download_size_bytes: u32,
    /// Whether it's already installed in the managed SDK.
    pub installed: bool,
    /// Whether it bundles the Play Store.
    pub has_play_store: bool,
}

/// Today's real system-image catalog (Google's per-tag `sys-img2-3.xml` manifests), merged with
/// local install state.
#[tauri::command]
#[specta::specta]
pub async fn list_images(app: AppHandle) -> Result<Vec<ImageInfo>, IpcError> {
    let provider = provider(&app).await?;

    let manifests = try_join_all(
        emu_android::sysimg::MANIFEST_URLS
            .iter()
            .map(|url| fetch_bytes(url)),
    )
    .await?;

    let mut entries = Vec::new();
    for xml in &manifests {
        entries.extend(emu_android::sysimg::parse(xml).map_err(IpcError::from)?);
    }
    entries.sort_by_key(|e| e.coord.to_string());
    entries.dedup_by(|a, b| a.coord == b.coord);

    let mut images = Vec::with_capacity(entries.len());
    for entry in &entries {
        let installed = provider
            .is_image_installed(entry.coord)
            .await
            .unwrap_or(false);
        images.push(ImageInfo {
            coord: entry.coord.to_string(),
            api: entry.coord.api,
            android_version: emu_android::sysimg::android_version_name(entry.coord.api),
            image_type: entry.coord.image_type.tag().to_string(),
            abi: entry.coord.abi.tag().to_string(),
            revision: entry.revision.clone(),
            download_size_bytes: u32::try_from(entry.size_bytes.unwrap_or(0)).unwrap_or(u32::MAX),
            installed,
            has_play_store: entry.coord.image_type.has_play_store(),
        });
    }
    images.sort_by(|a, b| {
        b.api
            .cmp(&a.api)
            .then_with(|| a.image_type.cmp(&b.image_type))
            .then_with(|| a.abi.cmp(&b.abi))
    });
    Ok(images)
}

// ---------------------------------------------------------------------------
// Tracked emulators (Dashboard)
// ---------------------------------------------------------------------------

/// A tracked emulator plus its live run state.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorInfo {
    pub id: String,
    pub avd_name: String,
    pub display_name: String,
    /// `stopped` / `booting` / `running` / `error`.
    pub state: String,
    pub adb_serial: Option<String>,
}

fn run_state_label(state: RunState) -> &'static str {
    match state {
        RunState::Stopped => "stopped",
        RunState::Booting => "booting",
        RunState::Running => "running",
        _ => "error",
    }
}

impl From<&TrackedEmulator> for EmulatorInfo {
    fn from(tracked: &TrackedEmulator) -> Self {
        Self {
            id: tracked.id.to_string(),
            avd_name: tracked.avd_name.clone(),
            display_name: tracked.display_name.clone(),
            state: run_state_label(tracked.state).to_string(),
            adb_serial: tracked.adb_serial.clone(),
        }
    }
}

/// Every registry-tracked emulator with its live state — what the Dashboard polls.
#[tauri::command]
#[specta::specta]
pub async fn list_emulators(app: AppHandle) -> Result<Vec<EmulatorInfo>, IpcError> {
    let provider = provider(&app).await?;
    let tracked = provider.tracked_states().await.map_err(IpcError::from)?;
    Ok(tracked.iter().map(EmulatorInfo::from).collect())
}

// ---------------------------------------------------------------------------
// create / launch / stop  +  the job event
// ---------------------------------------------------------------------------

/// One update from a `create_emulator`/`launch_emulator` run. Same tagged shape as
/// `toolchain::BootstrapProgressKind` (`docs/architecture.md` §4's `job://*` events, collapsed);
/// the `jobId` on [`EmulatorJob`] says which run it belongs to.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EmulatorJobKind {
    /// A phase and/or percentage update.
    Progress {
        phase: Option<String>,
        pct: Option<u8>,
    },
    /// One log line to append to the run's tail.
    Log { line: String },
    /// The run reached a terminal state.
    Done { ok: bool, error: Option<IpcError> },
}

/// Live progress for an emulator create/launch run, emitted as `job://emulator`.
#[derive(Debug, Clone, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "job://emulator")]
#[serde(rename_all = "camelCase")]
pub struct EmulatorJob {
    /// `create:<avdName>` or `launch:<id>` — the frontend filters its progress panel by this.
    pub job_id: String,
    pub payload: EmulatorJobKind,
}

fn emit_job(app: &AppHandle, job_id: &str, payload: EmulatorJobKind) {
    let _ = EmulatorJob {
        job_id: job_id.to_string(),
        payload,
    }
    .emit(app);
}

fn job_handle(app: &AppHandle, job_id: &str) -> JobHandle {
    let sink_app = app.clone();
    let sink_job_id = job_id.to_string();
    JobHandle::new(
        JobId(job_id.to_string()),
        Box::new(move |p: Progress| {
            if let Some(line) = p.log_line {
                emit_job(&sink_app, &sink_job_id, EmulatorJobKind::Log { line });
            }
            if p.phase.is_some() || p.pct.is_some() {
                emit_job(
                    &sink_app,
                    &sink_job_id,
                    EmulatorJobKind::Progress {
                        phase: p.phase,
                        pct: p.pct,
                    },
                );
            }
        }),
    )
}

/// `[a-zA-Z0-9._-]` only — the character class `avdmanager -n` accepts; everything else becomes
/// `_`.
fn sanitize_avd_name(display_name: &str) -> String {
    let name: String = display_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        "avd".to_string()
    } else {
        name
    }
}

/// Request body for [`create_emulator`].
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmulatorRequest {
    /// Display name; the on-disk AVD name is a sanitized version of it.
    pub name: String,
    /// `DeviceInfo.id`.
    pub device_id: String,
    /// `ImageInfo.coord`.
    pub image_coord: String,
    pub ram_mb: u32,
    pub storage_mb: u32,
    /// Also launch it once created ("Create & launch").
    pub launch: bool,
}

/// Ensure the chosen image is installed, create the AVD, optionally launch it. Streams progress
/// on `job://emulator` with `jobId` `create:<avdName>`. Returns the new tracked id.
#[tauri::command]
#[specta::specta]
pub async fn create_emulator(
    app: AppHandle,
    request: CreateEmulatorRequest,
) -> Result<String, IpcError> {
    let display_name = request.name.trim().to_string();
    if display_name.is_empty() {
        return Err(IpcError::new("invalid", "the emulator needs a name"));
    }
    let coord: ImageCoord = request.image_coord.parse().map_err(IpcError::from)?;
    let avd_name = sanitize_avd_name(&display_name);
    let job_id = format!("create:{avd_name}");
    let provider = provider(&app).await?;
    let job = job_handle(&app, &job_id);

    let outcome: Result<EmulatorId, CoreError> = async {
        job.report(Progress::log(format!(
            "ensuring system image {coord} is installed (a first-time download can take several \
             minutes)"
        )));
        provider.ensure_image(coord, &job).await?;

        let spec = CreateSpec {
            avd_name: avd_name.clone(),
            display_name: display_name.clone(),
            device_profile_id: request.device_id.clone(),
            image_coord: coord,
            hardware: Hardware {
                ram_mb: request.ram_mb.max(512),
                storage_mb: request.storage_mb.max(1024),
                ..Hardware::default()
            },
        };
        job.report(Progress::log(format!("creating AVD {avd_name}")));
        let id = provider.create(spec).await?;

        if request.launch {
            job.report(Progress::log("launching…"));
            provider
                .launch(id.clone(), LaunchOpts::default(), &job)
                .await?;
        }
        Ok(id)
    }
    .await;

    match outcome {
        Ok(id) => {
            emit_job(
                &app,
                &job_id,
                EmulatorJobKind::Done {
                    ok: true,
                    error: None,
                },
            );
            Ok(id.to_string())
        }
        Err(e) => {
            let ipc = IpcError::from(e);
            emit_job(
                &app,
                &job_id,
                EmulatorJobKind::Done {
                    ok: false,
                    error: Some(ipc.clone()),
                },
            );
            Err(ipc)
        }
    }
}

/// Launch an already-created emulator and wait for it to boot. Streams on `job://emulator` with
/// `jobId` `launch:<id>`.
#[tauri::command]
#[specta::specta]
pub async fn launch_emulator(app: AppHandle, id: String) -> Result<(), IpcError> {
    let job_id = format!("launch:{id}");
    let provider = provider(&app).await?;
    let job = job_handle(&app, &job_id);

    match provider
        .launch(EmulatorId(id), LaunchOpts::default(), &job)
        .await
    {
        Ok(_handle) => {
            emit_job(
                &app,
                &job_id,
                EmulatorJobKind::Done {
                    ok: true,
                    error: None,
                },
            );
            Ok(())
        }
        Err(e) => {
            let ipc = IpcError::from(e);
            emit_job(
                &app,
                &job_id,
                EmulatorJobKind::Done {
                    ok: false,
                    error: Some(ipc.clone()),
                },
            );
            Err(ipc)
        }
    }
}

/// Stop a running emulator (`adb emu kill`).
#[tauri::command]
#[specta::specta]
pub async fn stop_emulator(app: AppHandle, id: String) -> Result<(), IpcError> {
    let provider = provider(&app).await?;
    provider.stop(EmulatorId(id)).await.map_err(IpcError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_avd_name_replaces_disallowed_chars() {
        assert_eq!(sanitize_avd_name("Pixel 6 · API 34"), "Pixel_6___API_34");
        assert_eq!(sanitize_avd_name("clean_name-1.0"), "clean_name-1.0");
        assert_eq!(sanitize_avd_name("  "), "__");
    }

    #[test]
    fn run_state_label_covers_every_variant() {
        assert_eq!(run_state_label(RunState::Stopped), "stopped");
        assert_eq!(run_state_label(RunState::Booting), "booting");
        assert_eq!(run_state_label(RunState::Running), "running");
        assert_eq!(run_state_label(RunState::Error), "error");
    }

    #[test]
    fn form_factor_label_covers_every_variant() {
        assert_eq!(form_factor_label(FormFactor::Phone), "phone");
        assert_eq!(form_factor_label(FormFactor::Foldable), "foldable");
        assert_eq!(form_factor_label(FormFactor::Automotive), "automotive");
    }
}
