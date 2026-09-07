//! Toolchain manager commands (milestone M1, task 0013): the real component catalog + installed
//! state for the Dependencies screen, and running the real `bootstrap()` with live progress.
//!
//! This is the first place any of task 0011's real port impls (`crate::ports`) are actually
//! constructed and called — see that module's doc comment for why the `#[allow(dead_code)]` it
//! carried until now is gone.

use std::path::PathBuf;

use emu_core::model::component::{Component, ComponentId, HostArch, HostOs};
use emu_core::model::job::{JobHandle, JobId, Progress};
use emu_core::toolchain::{self, BootstrapPorts, InstalledState, SdkSource};
use serde::Serialize;
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::ipc_error::IpcError;
use crate::ports::{NativeDownloader, NativeFs, NativeProcessRunner};

/// One component as shown on the Dependencies screen: catalog info merged with real install
/// state (`installed`/`source` come from `emu_core::toolchain::InstalledState`, scanned fresh on
/// every call — never cached, per the same rule `SystemImage::installed` follows).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ComponentInfo {
    /// The exact `sdkmanager` package path (`ComponentId::repo_path`) — a stable id.
    pub id: String,
    /// Human-readable label for the UI.
    pub name: String,
    /// Package revision from the catalog.
    pub version: String,
    /// Archive size in bytes. `u32` (not the catalog's `u64`) because specta-typescript refuses
    /// to export a bigint-risking `u64` to a plain JS `number` — every real M1 archive is well
    /// under 4 GiB, and even `docs/spec.md`'s largest future system images (~3.5 GB) fit, so this
    /// is a safe, deliberate narrowing, saturating at `u32::MAX` in the (currently impossible)
    /// case of something bigger.
    pub size_bytes: u32,
    /// Whether it's installed anywhere (the app's managed `sdk/` dir, or an existing system SDK).
    pub installed: bool,
    /// Where it was found — `"app-managed"` or `"system:<path>"` — `None` when not installed.
    pub source: Option<String>,
}

/// One update from a `bootstrap_toolchain` run. Mirrors `docs/architecture.md` §4's
/// `job://progress`/`job://log`/`job://done` shapes, collapsed into one typed event (there is only
/// ever one toolchain-bootstrap job at a time — a real per-job registry is M3's "Registry &
/// reliable tracking" milestone, not needed here).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BootstrapProgressKind {
    /// A phase and/or percentage update.
    Progress {
        phase: Option<String>,
        pct: Option<u8>,
    },
    /// One log line to append to the run's tail.
    Log { line: String },
    /// The job reached a terminal state.
    Done { ok: bool, error: Option<IpcError> },
}

/// Live progress for a `bootstrap_toolchain` run, emitted as `job://bootstrap`.
#[derive(Debug, Clone, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "job://bootstrap")]
#[serde(rename_all = "camelCase")]
pub struct BootstrapProgress {
    /// Always [`JOB_ID`] today — carried on the payload anyway so the frontend never has to know
    /// that, and so a future real job registry is a backward-compatible change, not a breaking one.
    pub job_id: String,
    pub payload: BootstrapProgressKind,
}

/// The one toolchain-bootstrap job id. There is only ever one such job in flight at a time (the
/// Dependencies screen's single "Install" action), so a fixed id is simpler than generating one
/// per call.
const JOB_ID: &str = "toolchain-bootstrap";

/// A human label for the UI. Matches on the real `sdkmanager` package path (not the enum
/// discriminant directly — `ComponentId` is `#[non_exhaustive]`, so a match on it from outside
/// `emu-core` needs a catch-all anyway; matching the stable path string doubles as that catch-all
/// without silently mis-naming a variant this crate hasn't been updated to know about).
fn component_name(id: ComponentId) -> String {
    match id.repo_path() {
        "cmdline-tools;latest" => "cmdline-tools".to_string(),
        "platform-tools" => "platform-tools (adb)".to_string(),
        "emulator" => "emulator".to_string(),
        other => other.to_string(),
    }
}

fn source_label(source: &SdkSource) -> String {
    match source {
        SdkSource::AppManaged => "app-managed".to_string(),
        SdkSource::System(path) => format!("system:{}", path.display()),
    }
}

fn host() -> Result<(HostOs, HostArch), IpcError> {
    let os = HostOs::current().ok_or_else(|| {
        IpcError::new(
            "unsupported",
            "this operating system isn't supported by the Android SDK",
        )
    })?;
    let arch = HostArch::current().ok_or_else(|| {
        IpcError::new(
            "unsupported",
            "this CPU architecture isn't supported by the Android SDK",
        )
    })?;
    Ok((os, arch))
}

/// Fetch and parse today's real component catalog from Google's repository manifest
/// (`emu_android::catalog`, task 0010) for the current host.
async fn resolve_catalog(os: HostOs, arch: HostArch) -> Result<Vec<Component>, IpcError> {
    let url = format!("{}repository2-3.xml", emu_android::catalog::BASE_URL);
    let xml = reqwest::get(&url)
        .await
        .map_err(|e| {
            IpcError::new(
                "download_failed",
                format!("fetching the component catalog: {e}"),
            )
        })?
        .bytes()
        .await
        .map_err(|e| {
            IpcError::new(
                "download_failed",
                format!("reading the component catalog: {e}"),
            )
        })?;
    emu_android::catalog::parse(&xml, os, arch).map_err(IpcError::from)
}

/// The app's data directory — the parent of the managed `sdk/` subtree `InstalledState`/
/// `bootstrap` work under.
fn app_data_dir(app: &AppHandle) -> Result<PathBuf, IpcError> {
    app.path()
        .app_data_dir()
        .map_err(|e| IpcError::new("fs_error", format!("resolving the app data directory: {e}")))
}

fn env_lookup(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Scan real installed state: the app-managed `sdk/` dir and any existing system SDK.
async fn scan_installed(
    app: &AppHandle,
    os: HostOs,
) -> Result<(PathBuf, InstalledState), IpcError> {
    let data_dir = app_data_dir(app)?;
    let sdk_dir = data_dir.join("sdk");
    let fs = NativeFs;
    let state = toolchain::scan(&fs, &sdk_dir, os, env_lookup)
        .await
        .map_err(IpcError::from)?;
    Ok((data_dir, state))
}

/// The real component catalog, merged with real installed state — what the Dependencies screen
/// renders.
#[tauri::command]
#[specta::specta]
pub async fn list_components(app: AppHandle) -> Result<Vec<ComponentInfo>, IpcError> {
    let (os, arch) = host()?;
    let catalog = resolve_catalog(os, arch).await?;
    let (_data_dir, state) = scan_installed(&app, os).await?;

    Ok(catalog
        .into_iter()
        .map(|c| {
            let location = state.location_of(c.id);
            ComponentInfo {
                id: c.id.repo_path().to_string(),
                name: component_name(c.id),
                version: c.version,
                size_bytes: u32::try_from(c.size_bytes).unwrap_or(u32::MAX),
                installed: location.is_some(),
                source: location.map(|l| source_label(&l.source)),
            }
        })
        .collect())
}

fn emit_progress(app: &AppHandle, payload: BootstrapProgressKind) {
    let event = BootstrapProgress {
        job_id: JOB_ID.to_string(),
        payload,
    };
    // A dropped event (no listener yet, or the window closed) isn't fatal to the bootstrap run
    // itself — only the final `Result` returned to the caller is load-bearing.
    let _ = event.emit(app);
}

/// Run the real `bootstrap()` (task 0012) for every M1 component still missing, reporting
/// progress on `job://bootstrap` as it goes.
#[tauri::command]
#[specta::specta]
pub async fn bootstrap_toolchain(app: AppHandle) -> Result<(), IpcError> {
    let (os, arch) = host()?;
    let catalog = resolve_catalog(os, arch).await?;
    let (data_dir, state) = scan_installed(&app, os).await?;
    run_bootstrap(&app, &data_dir, &catalog, &state, os).await
}

/// Install a **single** SDK component by its `sdkmanager` package path (`ComponentInfo.id`, e.g.
/// `platform-tools`). Same `job://bootstrap` progress channel as [`bootstrap_toolchain`] — only
/// one such job runs at a time. A component that's already installed is a clean no-op.
#[tauri::command]
#[specta::specta]
pub async fn install_component(app: AppHandle, component_id: String) -> Result<(), IpcError> {
    let (os, arch) = host()?;
    let catalog = resolve_catalog(os, arch).await?;
    let wanted: Vec<Component> = catalog
        .into_iter()
        .filter(|c| c.id.repo_path() == component_id)
        .collect();
    if wanted.is_empty() {
        return Err(IpcError::new(
            "not_found",
            format!("no SDK component with id '{component_id}'"),
        ));
    }
    let (data_dir, state) = scan_installed(&app, os).await?;
    run_bootstrap(&app, &data_dir, &wanted, &state, os).await
}

/// Shared tail of `bootstrap_toolchain` / `install_component`: build the ports + job, run
/// `toolchain::bootstrap` for `wanted`, and emit the terminal `job://bootstrap` event.
async fn run_bootstrap(
    app: &AppHandle,
    data_dir: &std::path::Path,
    wanted: &[Component],
    state: &InstalledState,
    os: HostOs,
) -> Result<(), IpcError> {
    let fs = NativeFs;
    let downloader = NativeDownloader::new();
    let process = NativeProcessRunner;
    let ports = BootstrapPorts {
        fs: &fs,
        downloader: &downloader,
        process: &process,
    };

    let sink_app = app.clone();
    let job = JobHandle::new(
        JobId(JOB_ID.to_string()),
        Box::new(move |p: Progress| {
            if let Some(line) = p.log_line {
                emit_progress(&sink_app, BootstrapProgressKind::Log { line });
            }
            if p.phase.is_some() || p.pct.is_some() {
                emit_progress(
                    &sink_app,
                    BootstrapProgressKind::Progress {
                        phase: p.phase,
                        pct: p.pct,
                    },
                );
            }
        }),
    );

    match toolchain::bootstrap(data_dir, wanted, state, os, &ports, &job).await {
        Ok(()) => {
            emit_progress(
                app,
                BootstrapProgressKind::Done {
                    ok: true,
                    error: None,
                },
            );
            Ok(())
        }
        Err(e) => {
            let ipc = IpcError::from(e);
            emit_progress(
                app,
                BootstrapProgressKind::Done {
                    ok: false,
                    error: Some(ipc.clone()),
                },
            );
            Err(ipc)
        }
    }
}
