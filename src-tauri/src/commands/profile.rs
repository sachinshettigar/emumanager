//! Profile commands (milestone M4, task 0023): inspect / apply / export a `.emuprofile`, and the
//! saved-profiles list.
//!
//! `inspect_profile` parses + validates + `emu_core::profile::resolve`s against real install
//! state; `apply_profile` runs the resulting plan through the shared [`AndroidProvider`], stamping
//! `EmulatorSource::Imported`. Same job-event channel (`job://emulator`) as `create_emulator`.

use emu_core::model::emulator::{Emulator, EmulatorId, EmulatorSource};
use emu_core::model::image::ImageCoord;
use emu_core::model::job::Progress;
use emu_core::model::plan::{CreateSpec, RequirementStatus};
use emu_core::model::profile::{EmuProfile, SCHEMA_VERSION};
use emu_core::provider::{LaunchOpts, Provider as _};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::emulator::{emit_job, job_handle, EmulatorJobKind};
use crate::ipc_error::IpcError;
use crate::provider_state::ManagedProvider;

// ---------------------------------------------------------------------------
// Parsing with specific, user-facing rejection messages
// ---------------------------------------------------------------------------

/// Parse `bytes` into an [`EmuProfile`], with a *specific* error for each way it can be wrong —
/// the frontend shows `message` verbatim.
fn parse_profile(bytes: &[u8]) -> Result<EmuProfile, IpcError> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| {
        IpcError::new(
            "invalid",
            "this file isn't valid JSON — a .emuprofile is a small JSON recipe",
        )
    })?;

    let Some(version) = value.get("schemaVersion") else {
        return Err(IpcError::new(
            "invalid",
            "this doesn't look like a .emuprofile (no \"schemaVersion\" field)",
        ));
    };
    if version.as_str() != Some(SCHEMA_VERSION) {
        return Err(IpcError::new(
            "unsupported",
            format!(
                "unsupported .emuprofile version {version} — this app understands \"{SCHEMA_VERSION}\""
            ),
        ));
    }
    if let Some(platform) = value.get("platform").and_then(serde_json::Value::as_str) {
        if platform != "android" {
            return Err(IpcError::new(
                "unsupported",
                format!("this profile targets \"{platform}\", not Android"),
            ));
        }
    }

    let profile: EmuProfile = serde_json::from_value(value).map_err(|e| {
        IpcError::new(
            "invalid",
            format!("this .emuprofile has a bad or missing field: {e}"),
        )
    })?;
    profile.validate().map_err(IpcError::from)?;
    Ok(profile)
}

/// The download size of `coord`, looked up in Google's `sys-img2-3.xml` manifests. `None` when the
/// coord isn't in any manifest or the fetch fails (the diff then just omits a size).
async fn image_size_bytes(coord: ImageCoord) -> Option<u64> {
    let manifests = futures_util::future::join_all(
        emu_android::sysimg::MANIFEST_URLS
            .iter()
            .map(|url| async move { reqwest::get(*url).await.ok()?.bytes().await.ok() }),
    )
    .await;
    for xml in manifests.into_iter().flatten() {
        if let Ok(entries) = emu_android::sysimg::parse(xml.as_ref()) {
            if let Some(entry) = entries.into_iter().find(|e| e.coord == coord) {
                return entry.size_bytes;
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// inspect
// ---------------------------------------------------------------------------

/// One line of the import requirement table.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRequirement {
    pub label: String,
    /// `true` when already installed locally.
    pub present: bool,
    /// Bytes to download when not present (0 when present or unknown), saturating `u32`.
    pub download_bytes: u32,
}

/// The preview shown before "Apply".
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInspection {
    pub name: String,
    pub description: String,
    pub device_label: String,
    pub image_label: String,
    /// `sdkmanager` package path of the system image the recipe needs.
    pub image_coord: String,
    pub requirements: Vec<ProfileRequirement>,
    pub total_download_bytes: u32,
    /// `true` when nothing needs downloading — Apply is a straight create.
    pub ready: bool,
}

/// Parse + validate + resolve a dropped `.emuprofile` into a preview.
#[tauri::command]
#[specta::specta]
pub async fn inspect_profile(
    mgr: State<'_, ManagedProvider>,
    bytes: Vec<u8>,
) -> Result<ProfileInspection, IpcError> {
    let profile = parse_profile(&bytes)?;
    let provider = mgr.get().await?;
    let components = provider.installed_state().await.map_err(IpcError::from)?;
    let coord = profile.image_coord();
    let image_installed = provider.is_image_installed(coord).await.unwrap_or(false);
    let size = image_size_bytes(coord).await;

    let plan = emu_core::profile::resolve(&profile, &components, image_installed, size)
        .map_err(IpcError::from)?;

    let requirements: Vec<ProfileRequirement> = plan
        .diff
        .iter()
        .map(|r| match r.status {
            RequirementStatus::Present => ProfileRequirement {
                label: r.label.clone(),
                present: true,
                download_bytes: 0,
            },
            RequirementStatus::NeedsDownload { size_bytes } => ProfileRequirement {
                label: r.label.clone(),
                present: false,
                download_bytes: u32::try_from(size_bytes.unwrap_or(0)).unwrap_or(u32::MAX),
            },
            // `RequirementStatus` is `#[non_exhaustive]`; treat a future variant as "needs work".
            _ => ProfileRequirement {
                label: r.label.clone(),
                present: false,
                download_bytes: 0,
            },
        })
        .collect();

    Ok(ProfileInspection {
        name: profile.name.clone(),
        description: profile.description.clone().unwrap_or_default(),
        device_label: profile.device.profile.clone(),
        image_label: format!(
            "Android API {} · {} · {}",
            profile.image.api,
            coord.image_type.tag(),
            coord.abi.tag()
        ),
        image_coord: coord.to_string(),
        requirements,
        total_download_bytes: u32::try_from(plan.total_download_bytes()).unwrap_or(u32::MAX),
        ready: plan.is_ready(),
    })
}

// ---------------------------------------------------------------------------
// apply
// ---------------------------------------------------------------------------

/// Import a `.emuprofile`: ensure its image, create the AVD as `Imported`, optionally launch.
/// Streams on `job://emulator` with `jobId` `apply:<avdName>`. Returns the new tracked id.
#[tauri::command]
#[specta::specta]
pub async fn apply_profile(
    app: AppHandle,
    mgr: State<'_, ManagedProvider>,
    bytes: Vec<u8>,
    launch: bool,
) -> Result<String, IpcError> {
    let profile = parse_profile(&bytes)?;
    let provider = mgr.get().await?;
    let coord = profile.image_coord();
    let avd_name = emu_core::profile::sanitize_avd_name(&profile.name);
    let job_id = format!("apply:{avd_name}");
    let job = job_handle(&app, &job_id);

    let spec = CreateSpec {
        avd_name,
        display_name: profile.name.clone(),
        device_profile_id: profile.device.profile.clone(),
        image_coord: coord,
        hardware: profile.hardware(),
    };

    let outcome: Result<EmulatorId, IpcError> = async {
        job.report(Progress::log(format!(
            "importing profile '{}' — ensuring system image {coord}",
            profile.name
        )));
        provider
            .ensure_image(coord, &job)
            .await
            .map_err(IpcError::from)?;
        let id = provider.create(spec).await.map_err(IpcError::from)?;
        provider
            .set_source(
                &id,
                &EmulatorSource::Imported {
                    profile_id: profile.name.clone(),
                    origin_label: "imported .emuprofile".to_string(),
                },
            )
            .await
            .map_err(IpcError::from)?;
        if launch {
            job.report(Progress::log("launching…"));
            provider
                .launch(id.clone(), LaunchOpts::default(), &job)
                .await
                .map_err(IpcError::from)?;
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
        Err(ipc) => {
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

// ---------------------------------------------------------------------------
// export
// ---------------------------------------------------------------------------

/// Build the pretty-printed `.emuprofile` JSON for a tracked emulator.
async fn build_profile_json(
    provider: &emu_android::provider::AndroidProvider,
    id: String,
) -> Result<String, IpcError> {
    let row = provider
        .detail(&EmulatorId(id))
        .await
        .map_err(IpcError::from)?;
    let image_coord = row.image_coord.ok_or_else(|| {
        IpcError::new(
            "invalid",
            "this emulator has no known system image, so it can't be saved as a profile — \
             reconcile or recreate it first",
        )
    })?;
    let emulator = Emulator {
        id: row.id,
        avd_name: row.avd_name,
        display_name: row.display_name,
        device_profile_id: row.device_profile_id,
        image_coord,
        hardware: row.hardware,
        source: row.source,
        tags: row.tags,
        notes: row.notes,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };
    Ok(EmuProfile::from_emulator(&emulator).to_json_pretty())
}

/// The tracked emulator `id` as a pretty-printed `.emuprofile` JSON string (for the clipboard).
#[tauri::command]
#[specta::specta]
pub async fn export_profile(
    mgr: State<'_, ManagedProvider>,
    id: String,
) -> Result<String, IpcError> {
    let provider = mgr.get().await?;
    build_profile_json(&provider, id).await
}

/// Write the tracked emulator `id` as an `.emuprofile` file at `path` — the location the user
/// picked in the native Save dialog (`@tauri-apps/plugin-dialog`'s `save()`). The mirror of the
/// Profiles screen's drag-in import.
#[tauri::command]
#[specta::specta]
pub async fn export_profile_to_path(
    mgr: State<'_, ManagedProvider>,
    id: String,
    path: String,
) -> Result<(), IpcError> {
    let provider = mgr.get().await?;
    let json = build_profile_json(&provider, id).await?;
    std::fs::write(&path, json)
        .map_err(|e| IpcError::new("fs_error", format!("writing {path}: {e}")))
}

// ---------------------------------------------------------------------------
// saved profiles
// ---------------------------------------------------------------------------

/// One saved recipe in the list.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub name: String,
    pub description: String,
    /// RFC 3339.
    pub created_at: String,
}

/// Validate and save a `.emuprofile` to the local list (keyed by its `name`).
#[tauri::command]
#[specta::specta]
pub async fn save_profile(mgr: State<'_, ManagedProvider>, bytes: Vec<u8>) -> Result<(), IpcError> {
    let profile = parse_profile(&bytes)?;
    let provider = mgr.get().await?;
    provider
        .registry()
        .save_profile(
            &profile.name,
            profile.description.as_deref().unwrap_or(""),
            &profile.to_json_pretty(),
        )
        .await
        .map_err(IpcError::from)
}

/// Every saved profile, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_profiles(
    mgr: State<'_, ManagedProvider>,
) -> Result<Vec<ProfileSummary>, IpcError> {
    let provider = mgr.get().await?;
    let rows = provider
        .registry()
        .list_profiles()
        .await
        .map_err(IpcError::from)?;
    Ok(rows
        .into_iter()
        .map(|(name, description, created_at)| ProfileSummary {
            name,
            description,
            created_at,
        })
        .collect())
}

/// The JSON body of a saved profile — for "Apply" of a saved entry (feed it back to `apply_profile`).
#[tauri::command]
#[specta::specta]
pub async fn get_saved_profile(
    mgr: State<'_, ManagedProvider>,
    name: String,
) -> Result<String, IpcError> {
    let provider = mgr.get().await?;
    provider
        .registry()
        .get_profile(&name)
        .await
        .map_err(IpcError::from)?
        .ok_or_else(|| IpcError::new("not_found", format!("no saved profile named '{name}'")))
}

/// Delete a saved profile.
#[tauri::command]
#[specta::specta]
pub async fn delete_profile(mgr: State<'_, ManagedProvider>, name: String) -> Result<(), IpcError> {
    let provider = mgr.get().await?;
    provider
        .registry()
        .delete_profile(&name)
        .await
        .map_err(IpcError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(kind: &str, name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../schemas/emuprofile/fixtures/{kind}/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read fixture")
    }

    #[test]
    fn parse_profile_accepts_every_valid_fixture() {
        for name in ["minimal.json", "full.json", "custom-device.json"] {
            parse_profile(&read_fixture("valid", name))
                .unwrap_or_else(|e| panic!("{name}: {}", e.message));
        }
    }

    #[test]
    fn parse_profile_gives_specific_messages() {
        let not_json = parse_profile(b"not json at all").unwrap_err();
        assert!(not_json.message.contains("valid JSON"));

        let no_version = parse_profile(br#"{"name":"x"}"#).unwrap_err();
        assert!(no_version.message.contains("schemaVersion"));

        let bad_version =
            parse_profile(br#"{"schemaVersion":"9.9","platform":"android"}"#).unwrap_err();
        assert_eq!(bad_version.code, "unsupported");
        assert!(bad_version.message.contains("9.9"));

        let wrong_platform =
            parse_profile(br#"{"schemaVersion":"1.0","platform":"ios"}"#).unwrap_err();
        assert_eq!(wrong_platform.code, "unsupported");
        assert!(wrong_platform.message.contains("ios"));

        let wrong_platform_fixture = parse_profile(&read_fixture("invalid", "wrong-platform.json"));
        assert!(wrong_platform_fixture.is_err());

        // `unknown-field.json` — serde `deny_unknown_fields` rejects it.
        let unknown_field = parse_profile(&read_fixture("invalid", "unknown-field.json"));
        assert!(unknown_field.is_err());

        // Note: `bad-abi.json` (schema enum violation on `abi`) is NOT caught here — the domain
        // `Abi` type accepts more ABIs than the schema's two. Full JSON-Schema enforcement is the
        // frontend's `ajv` pass (`scripts/validate-schema.mjs`); `parse_profile` is the serde +
        // domain-invariant backstop.
    }
}
