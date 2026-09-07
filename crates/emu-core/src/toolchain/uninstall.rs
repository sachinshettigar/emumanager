//! Removing a managed SDK component — the inverse of [`super::bootstrap`].
//!
//! Only ever touches the app-managed `<data_dir>/sdk/` tree. A component found in a *system* SDK
//! (an existing Android Studio install, `ANDROID_HOME`, …) is never modified — Emulator Studio
//! didn't put it there and won't take it away. `cmdline-tools` can't be removed this way either:
//! it *is* the `sdkmanager` doing the removing, and dropping it would strand the SDK with no way
//! to reinstall anything (resetting the app data directory is the escape hatch).
//!
//! `sdkmanager --uninstall <package> --sdk_root=<path>` is the documented removal command
//! (<https://developer.android.com/tools/sdkmanager> — "Uninstall packages"). It exits non-zero
//! and prints to stderr on failure, same contract as the install path in [`super::bootstrap`].

use std::path::Path;

use crate::error::{CoreError, Result};
use crate::model::component::{ComponentId, HostOs};
use crate::model::job::{JobHandle, Progress};
use crate::ports::{Command, ProcessRunner};
use crate::toolchain::installed_state::{binary_path, InstalledState, SdkSource};

/// Ports [`uninstall`] needs — just a process runner for `sdkmanager --uninstall`.
pub struct UninstallPorts<'a> {
    /// Runs `sdkmanager`.
    pub process: &'a dyn ProcessRunner,
}

/// Remove `target` from the app-managed SDK with `sdkmanager --uninstall`.
///
/// Errors:
/// - [`CoreError::Unsupported`] if `target` is [`ComponentId::CmdlineTools`] (see the module doc),
///   or if it is installed only in a system SDK.
/// - [`CoreError::NotFound`] if the command-line tools are not installed at all — nothing can run
///   `--uninstall`.
/// - [`CoreError::Process`] if `sdkmanager` exits non-zero.
///
/// A `target` that is not installed anywhere is a clean no-op (one log line, then `Ok`).
pub async fn uninstall(
    data_dir: &Path,
    target: ComponentId,
    state: &InstalledState,
    os: HostOs,
    ports: &UninstallPorts<'_>,
    job: &JobHandle,
) -> Result<()> {
    if target == ComponentId::CmdlineTools {
        return Err(CoreError::Unsupported(
            "the SDK command-line tools can't be removed from here — they're the component that \
             installs and removes every other one. Reset the app data directory to start clean."
                .to_string(),
        ));
    }

    let Some(location) = state.location_of(target) else {
        job.report(Progress::log(format!(
            "{} isn't installed; nothing to remove",
            target.repo_path()
        )));
        return Ok(());
    };

    if let SdkSource::System(root) = &location.source {
        return Err(CoreError::Unsupported(format!(
            "{} comes from a system Android SDK at {} — Emulator Studio won't modify an SDK it \
             didn't install. Remove it with that SDK's own tools if you need to.",
            target.repo_path(),
            root.display()
        )));
    }

    let cmdline = state
        .location_of(ComponentId::CmdlineTools)
        .ok_or_else(|| CoreError::NotFound {
            what: "cmdline-tools",
            name: "sdkmanager (needed to uninstall anything)".to_string(),
        })?;
    let sdkmanager = binary_path(&cmdline.sdk_root, ComponentId::CmdlineTools, os);
    let app_sdk_dir = data_dir.join("sdk");

    job.report(Progress::log(format!("removing {}", target.repo_path())));

    let cmd = Command::new(sdkmanager.display().to_string())
        .arg("--uninstall")
        .arg(target.repo_path())
        .arg(format!("--sdk_root={}", app_sdk_dir.display()));
    let output = ports.process.run(cmd).await?;

    if output.success() {
        job.report(Progress::log(format!("{} removed", target.repo_path())));
        Ok(())
    } else {
        Err(CoreError::Process {
            program: sdkmanager.display().to_string(),
            code: output.status,
            stderr: output.stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::job::JobId;
    use crate::ports::Output;
    use crate::testing::FakeProcessRunner;
    use crate::toolchain::installed_state::ComponentLocation;

    fn ok(stdout: &str) -> Output {
        Output {
            status: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn app_managed(id: ComponentId) -> ComponentLocation {
        ComponentLocation {
            id,
            sdk_root: PathBuf::from("/data/sdk"),
            source: SdkSource::AppManaged,
        }
    }

    fn job() -> JobHandle {
        JobHandle::noop(JobId("uninstall-test".to_string()))
    }

    #[tokio::test]
    async fn app_managed_component_is_removed_with_sdkmanager_uninstall() {
        let state = InstalledState::from_found(vec![
            app_managed(ComponentId::CmdlineTools),
            app_managed(ComponentId::Emulator),
        ]);
        let process = FakeProcessRunner::new().on_run("sdkmanager --uninstall emulator", ok(""));
        let ports = UninstallPorts { process: &process };

        uninstall(
            Path::new("/data"),
            ComponentId::Emulator,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect("uninstall");

        let calls = process.calls();
        assert_eq!(calls.len(), 1);
        let line = calls[0].display();
        assert!(line.contains("--uninstall"));
        assert!(line.contains("emulator"));
        assert!(line.contains("--sdk_root=/data/sdk"));
    }

    #[tokio::test]
    async fn removing_cmdline_tools_is_refused_without_touching_any_process() {
        let state = InstalledState::from_found(vec![app_managed(ComponentId::CmdlineTools)]);
        let process = FakeProcessRunner::new();
        let ports = UninstallPorts { process: &process };

        let err = uninstall(
            Path::new("/data"),
            ComponentId::CmdlineTools,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect_err("must refuse");
        assert!(matches!(err, CoreError::Unsupported(_)));
        assert_eq!(process.call_count(), 0);
    }

    #[tokio::test]
    async fn a_system_sourced_component_is_never_modified() {
        let state = InstalledState::from_found(vec![
            app_managed(ComponentId::CmdlineTools),
            ComponentLocation {
                id: ComponentId::Emulator,
                sdk_root: PathBuf::from("/opt/android-sdk"),
                source: SdkSource::System(PathBuf::from("/opt/android-sdk")),
            },
        ]);
        let process = FakeProcessRunner::new();
        let ports = UninstallPorts { process: &process };

        let err = uninstall(
            Path::new("/data"),
            ComponentId::Emulator,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect_err("system SDK is off-limits");
        assert!(matches!(err, CoreError::Unsupported(_)));
        assert_eq!(process.call_count(), 0);
    }

    #[tokio::test]
    async fn removing_something_not_installed_is_a_noop() {
        let state = InstalledState::from_found(vec![app_managed(ComponentId::CmdlineTools)]);
        let process = FakeProcessRunner::new();
        let ports = UninstallPorts { process: &process };

        uninstall(
            Path::new("/data"),
            ComponentId::Emulator,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect("no-op ok");
        assert_eq!(process.call_count(), 0);
    }

    #[tokio::test]
    async fn without_cmdline_tools_there_is_nothing_to_uninstall_with() {
        // emulator present but the command-line tools are gone — can't run `--uninstall`.
        let state = InstalledState::from_found(vec![app_managed(ComponentId::Emulator)]);
        let process = FakeProcessRunner::new();
        let ports = UninstallPorts { process: &process };

        let err = uninstall(
            Path::new("/data"),
            ComponentId::Emulator,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect_err("no sdkmanager");
        assert!(matches!(
            err,
            CoreError::NotFound {
                what: "cmdline-tools",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_nonzero_sdkmanager_exit_surfaces_as_a_process_error() {
        let state = InstalledState::from_found(vec![
            app_managed(ComponentId::CmdlineTools),
            app_managed(ComponentId::PlatformTools),
        ]);
        let process = FakeProcessRunner::new().on_run(
            "sdkmanager --uninstall platform-tools",
            Output {
                status: 1,
                stdout: String::new(),
                stderr: "Warning: Package is in use".to_string(),
            },
        );
        let ports = UninstallPorts { process: &process };

        let err = uninstall(
            Path::new("/data"),
            ComponentId::PlatformTools,
            &state,
            HostOs::Linux,
            &ports,
            &job(),
        )
        .await
        .expect_err("process failed");
        assert!(matches!(err, CoreError::Process { code: 1, .. }));
    }
}
