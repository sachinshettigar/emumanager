//! [`resolve`] — `EmuProfile` + local state → [`Plan`].

use crate::model::component::ComponentId;
use crate::model::plan::{CreateSpec, Plan, Requirement, RequirementKind, RequirementStatus};
use crate::model::profile::EmuProfile;
use crate::toolchain::InstalledState;
use crate::Result;

/// Resolve a validated recipe into the requirement diff plus the [`CreateSpec`] to run once the
/// diff is satisfied.
///
/// `image_installed` / `image_size_bytes` are supplied by the caller — it knows the SDK filesystem
/// (`AndroidProvider::is_image_installed`) and the `sys-img2-3.xml` catalog. `platforms;android-NN`
/// is deliberately **not** a requirement: `avdmanager create avd` doesn't need it (verified against
/// a real SDK, task `0019` — creation succeeded before the platform was installed).
///
/// # Errors
/// [`CoreError::Invalid`](crate::CoreError::Invalid) when the profile isn't a supported Android v1
/// recipe (see [`EmuProfile::validate`]).
pub fn resolve(
    profile: &EmuProfile,
    components: &InstalledState,
    image_installed: bool,
    image_size_bytes: Option<u64>,
) -> Result<Plan> {
    profile.validate()?;
    let coord = profile.image_coord();

    let mut diff = Vec::with_capacity(4);
    for (kind, id) in [
        (RequirementKind::CmdlineTools, ComponentId::CmdlineTools),
        (RequirementKind::PlatformTools, ComponentId::PlatformTools),
        (RequirementKind::Emulator, ComponentId::Emulator),
    ] {
        diff.push(Requirement {
            kind,
            status: if components.is_installed(id) {
                RequirementStatus::Present
            } else {
                RequirementStatus::NeedsDownload { size_bytes: None }
            },
            coord: None,
            label: component_label(kind).to_string(),
        });
    }
    diff.push(Requirement {
        kind: RequirementKind::SystemImage,
        status: if image_installed {
            RequirementStatus::Present
        } else {
            RequirementStatus::NeedsDownload {
                size_bytes: image_size_bytes,
            }
        },
        coord: Some(coord),
        label: format!("System image · {coord}"),
    });

    let create_spec = CreateSpec {
        avd_name: sanitize_avd_name(&profile.name),
        display_name: profile.name.clone(),
        device_profile_id: profile.device.profile.clone(),
        image_coord: coord,
        hardware: profile.hardware(),
    };

    Ok(Plan { diff, create_spec })
}

/// Keep only the characters `avdmanager -n` accepts (`[A-Za-z0-9._-]`); everything else becomes
/// `_`. An empty result becomes `"avd"`.
#[must_use]
pub fn sanitize_avd_name(display_name: &str) -> String {
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

fn component_label(kind: RequirementKind) -> &'static str {
    match kind {
        RequirementKind::CmdlineTools => "Command-line tools",
        RequirementKind::PlatformTools => "Platform tools (adb)",
        RequirementKind::Emulator => "Emulator",
        RequirementKind::SystemImage => "System image",
        RequirementKind::Platform => "Platform",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::profile::EmuProfile;
    use crate::toolchain::{ComponentLocation, SdkSource};

    const VALID_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/emuprofile/fixtures/valid"
    );

    fn profile(name: &str) -> EmuProfile {
        let raw =
            std::fs::read_to_string(std::path::Path::new(VALID_DIR).join(name)).expect("read");
        serde_json::from_str(&raw).expect("parse")
    }

    fn all_installed() -> InstalledState {
        InstalledState::from_found(vec![
            ComponentLocation {
                id: ComponentId::CmdlineTools,
                source: SdkSource::AppManaged,
                sdk_root: std::path::PathBuf::from("/sdk"),
            },
            ComponentLocation {
                id: ComponentId::PlatformTools,
                source: SdkSource::AppManaged,
                sdk_root: std::path::PathBuf::from("/sdk"),
            },
            ComponentLocation {
                id: ComponentId::Emulator,
                source: SdkSource::AppManaged,
                sdk_root: std::path::PathBuf::from("/sdk"),
            },
        ])
    }

    #[test]
    fn everything_present_is_a_ready_plan() {
        let plan = resolve(&profile("full.json"), &all_installed(), true, None).expect("resolve");
        assert!(plan.is_ready());
        assert_eq!(plan.total_download_bytes(), 0);
        assert_eq!(plan.create_spec.display_name, "QA baseline");
        assert_eq!(plan.create_spec.avd_name, "QA_baseline");
        assert_eq!(plan.create_spec.image_coord.api, 33);
        // GiB → MiB.
        assert_eq!(plan.create_spec.hardware.storage_mb, 6 * 1024);
    }

    #[test]
    fn missing_image_and_tools_need_download() {
        let plan = resolve(
            &profile("minimal.json"),
            &InstalledState::from_found(vec![]),
            false,
            Some(1_500_000_000),
        )
        .expect("resolve");
        assert!(!plan.is_ready());
        assert_eq!(plan.total_download_bytes(), 1_500_000_000);
        let image = plan
            .diff
            .iter()
            .find(|r| r.kind == RequirementKind::SystemImage)
            .unwrap();
        assert!(matches!(
            image.status,
            RequirementStatus::NeedsDownload {
                size_bytes: Some(1_500_000_000)
            }
        ));
        assert!(image.coord.is_some());
    }

    #[test]
    fn non_android_profile_is_rejected() {
        let mut p = profile("minimal.json");
        p.platform = "ios".to_string();
        let err = resolve(&p, &all_installed(), true, None).unwrap_err();
        assert_eq!(err.code(), "invalid");
    }

    #[test]
    fn sanitize_avd_name_matches_the_shell_rule() {
        assert_eq!(sanitize_avd_name("Pixel 6 · API 34"), "Pixel_6___API_34");
        assert_eq!(sanitize_avd_name("clean-1.0"), "clean-1.0");
        assert_eq!(sanitize_avd_name(""), "avd");
    }
}
