//! [`AndroidProvider`] — the first real [`Provider`] implementation.
//!
//! `ensure_image` and `create` land in task `0015`; `launch`/`stop` in task `0016`; `list_devices`/
//! `list_images` (wiring task `0014`'s parsers to a real installed SDK) in task `0017`; `delete`/
//! `reconcile` are full M3 work (registry reconciliation). Until its own task, each stubs out with
//! [`CoreError::NotImplemented`] rather than a fake implementation.
//!
//! Design choice for `ensure_image` (see the task file's Notes for the real captures behind it):
//! system images are installed by asking the real `sdkmanager` to fetch and unpack them, the exact
//! same "don't reimplement what the real tool already does correctly" choice
//! `crates/emu-core/src/toolchain/bootstrap.rs` made for `platform-tools`/`emulator`. No
//! `Downloader`/zip-extraction code lives here at all.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use emu_core::error::{CoreError, Result};
use emu_core::model::component::{ComponentId, HostOs};
use emu_core::model::device::DeviceProfile;
use emu_core::model::emulator::{EmulatorId, LiveState};
use emu_core::model::image::{ImageCoord, ImageFilter, SystemImage};
use emu_core::model::job::{JobHandle, Progress};
use emu_core::model::plan::CreateSpec;
use emu_core::ports::{Command, Fs, ProcessRunner};
use emu_core::provider::{LaunchOpts, Provider, RunningHandle};
use emu_core::registry::Registry;
use emu_core::toolchain;

/// Feed this many `y\n` lines as stdin to a `sdkmanager`/`avdmanager` invocation that might hit a
/// license or confirmation prompt — bounded, not an infinite `yes |`, same rationale and count as
/// `crates/emu-core/src/toolchain/bootstrap.rs::LICENSE_ACCEPT_COUNT` (a real capture never needed
/// more than 7).
const PROMPT_ANSWER_COUNT: usize = 50;

/// Drives `sdkmanager`/`avdmanager`/`emulator`/`adb` via the injected [`ProcessRunner`].
pub struct AndroidProvider {
    process: Arc<dyn ProcessRunner>,
    fs: Arc<dyn Fs>,
    registry: Registry,
    /// The app's data dir (not the SDK dir itself — `<data_dir>/sdk` is the app-managed root;
    /// see `crates/emu-core/src/toolchain/bootstrap.rs`).
    data_dir: PathBuf,
    os: HostOs,
}

impl AndroidProvider {
    /// Build a provider over the given ports, app data dir, and host OS.
    #[must_use]
    pub fn new(
        process: Arc<dyn ProcessRunner>,
        fs: Arc<dyn Fs>,
        registry: Registry,
        data_dir: PathBuf,
        os: HostOs,
    ) -> Self {
        Self {
            process,
            fs,
            registry,
            data_dir,
            os,
        }
    }

    /// The SDK root to run `sdkmanager`/`avdmanager` under: wherever `cmdline-tools` was actually
    /// found (app-managed, or a real system SDK — task `0012`'s "reuse what's already there"
    /// rule), falling back to the app-managed dir if nothing has been scanned as installed yet
    /// (a real invocation would then fail with a clear "no such file" `Process` error, which is
    /// honest — there is truly nothing to run).
    async fn sdk_root(&self) -> Result<PathBuf> {
        let app_sdk_dir = self.data_dir.join("sdk");
        let state = toolchain::scan(self.fs.as_ref(), &app_sdk_dir, self.os, |k| {
            std::env::var(k).ok()
        })
        .await?;
        Ok(state
            .location_of(ComponentId::CmdlineTools)
            .map_or(app_sdk_dir, |l| l.sdk_root.clone()))
    }

    fn avdmanager_path(&self, sdk_root: &Path) -> PathBuf {
        let sdkmanager = toolchain::binary_path(sdk_root, ComponentId::CmdlineTools, self.os);
        let bin = if matches!(self.os, HostOs::Windows) {
            "avdmanager.bat"
        } else {
            "avdmanager"
        };
        sdkmanager.with_file_name(bin)
    }

    fn prompt_answer_stdin() -> Vec<u8> {
        "y\n".repeat(PROMPT_ANSWER_COUNT).into_bytes()
    }
}

/// `sdkmanager` mirrors a package's own `path` attribute directly onto its local install
/// directory, `;` replaced with `/` — the same convention already visible in this codebase for
/// `cmdline-tools;latest` → `cmdline-tools/latest`. Confirmed for system images by a real install
/// (task `0015`'s Notes): `sdkmanager "system-images;android-34;default;x86_64"` unpacks into
/// `<sdk_root>/system-images/android-34/default/x86_64/`.
fn image_dir(sdk_root: &Path, coord: ImageCoord) -> PathBuf {
    let mut dir = sdk_root.to_path_buf();
    for segment in coord.to_string().split(';') {
        dir.push(segment);
    }
    dir
}

/// Every real `sdkmanager`-installed package writes a `source.properties` at its root — the same
/// file `sdkmanager --list`/`avdmanager` themselves use to know a package is genuinely present,
/// not an invented marker (confirmed present in the real system-image archive this task
/// downloaded and inspected — see the task file's Notes).
fn image_marker_file(sdk_root: &Path, coord: ImageCoord) -> PathBuf {
    image_dir(sdk_root, coord).join("source.properties")
}

#[async_trait]
impl Provider for AndroidProvider {
    async fn list_devices(&self) -> Result<Vec<DeviceProfile>> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::list_devices (task 0017 wires emu_android::devices to a real \
             installed SDK)",
        ))
    }

    async fn list_images(&self, _filter: ImageFilter) -> Result<Vec<SystemImage>> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::list_images (task 0017 wires emu_android::sysimg to a real \
             installed SDK)",
        ))
    }

    /// No-op (fast success) when the image is already installed; otherwise asks the real
    /// `sdkmanager` to fetch, verify, and unpack it — see the module doc for why this doesn't use
    /// `Downloader`/zip code the way `cmdline-tools` bootstrap does.
    async fn ensure_image(&self, coord: ImageCoord, job: &JobHandle) -> Result<()> {
        let sdk_root = self.sdk_root().await?;
        if self.fs.exists(&image_marker_file(&sdk_root, coord)).await? {
            job.report(Progress::log(format!("{coord} already installed")));
            return Ok(());
        }

        let sdkmanager = toolchain::binary_path(&sdk_root, ComponentId::CmdlineTools, self.os);
        job.report(Progress::log(format!("installing {coord}")));
        let cmd = Command {
            stdin: Some(Self::prompt_answer_stdin()),
            ..Command::new(sdkmanager.display().to_string()).arg(coord.to_string())
        };
        let output = self.process.run(cmd).await?;
        if !output.success() {
            return Err(CoreError::Process {
                program: sdkmanager.display().to_string(),
                code: output.status,
                stderr: output.stderr,
            });
        }

        // `sdkmanager` can exit 0 having printed a warning instead of actually installing (a real
        // capture never showed this, but the check is cheap and turns a silent no-op into a clear
        // error instead of a confusing later "package not found" from `create`).
        if !self.fs.exists(&image_marker_file(&sdk_root, coord)).await? {
            return Err(CoreError::Invalid {
                what: "system image install",
                detail: format!(
                    "sdkmanager exited 0 but {coord} is not present under {}",
                    sdk_root.display()
                ),
            });
        }
        Ok(())
    }

    /// `avdmanager create avd -n <name> -k <image coord> -d <device id>` (real flags cited from a
    /// live `avdmanager create avd` — no `--help` flag exists for this subcommand, so the flag
    /// list came from the real "unknown flag" usage dump — see the task file's Notes). Always
    /// passing `-d` means the real "create a custom hardware profile?" prompt never appears, so no
    /// stdin is needed for a successful run; [`Self::prompt_answer_stdin`] is still fed in case a
    /// license happens not to be pre-accepted (harmless if unread).
    async fn create(&self, spec: CreateSpec) -> Result<EmulatorId> {
        let sdk_root = self.sdk_root().await?;
        let avdmanager = self.avdmanager_path(&sdk_root);

        let cmd = Command {
            stdin: Some(Self::prompt_answer_stdin()),
            ..Command::new(avdmanager.display().to_string())
                .arg("create")
                .arg("avd")
                .args(["-n", &spec.avd_name])
                .args(["-k", &spec.image_coord.to_string()])
                .args(["-d", &spec.device_profile_id])
        };
        let output = self.process.run(cmd).await?;
        if !output.success() {
            return Err(classify_create_avd_error(&spec, &output));
        }

        let id = EmulatorId::generate();
        self.registry
            .insert_emulator(id.as_str(), &spec.avd_name, &spec.display_name)
            .await?;
        Ok(id)
    }

    async fn launch(
        &self,
        _id: EmulatorId,
        _opts: LaunchOpts,
        _job: &JobHandle,
    ) -> Result<RunningHandle> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::launch (task 0016)",
        ))
    }

    async fn stop(&self, _id: EmulatorId) -> Result<()> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::stop (task 0016)",
        ))
    }

    async fn delete(&self, _id: EmulatorId, _wipe: bool) -> Result<()> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::delete (M3 — full registry reconciliation)",
        ))
    }

    async fn reconcile(&self) -> Result<Vec<LiveState>> {
        Err(CoreError::NotImplemented(
            "AndroidProvider::reconcile (M3 — full registry reconciliation)",
        ))
    }
}

/// Turn a real `avdmanager create avd` failure into a specific, actionable [`CoreError`]. All
/// three matched substrings are real, captured `stderr` text (task file Notes); anything else
/// falls back to the generic [`CoreError::Process`].
fn classify_create_avd_error(spec: &CreateSpec, output: &emu_core::ports::Output) -> CoreError {
    let stderr = &output.stderr;
    if stderr.contains("already exists") {
        CoreError::invalid(
            "avd name",
            format!(
                "'{}' already exists locally; pick a different name or delete it first",
                spec.avd_name
            ),
        )
    } else if stderr.contains("No device found matching") {
        CoreError::NotFound {
            what: "device profile",
            name: spec.device_profile_id.clone(),
        }
    } else if stderr.contains("Package path is not valid") {
        CoreError::NotFound {
            what: "system image",
            name: spec.image_coord.to_string(),
        }
    } else {
        CoreError::Process {
            program: "avdmanager".to_string(),
            code: output.status,
            stderr: stderr.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emu_core::model::image::{Abi, ImageType};
    use emu_core::model::plan::CreateSpec;
    use emu_core::ports::Output;
    use emu_core::testing::{FakeProcessRunner, InMemoryFs};

    fn ok(stdout: impl Into<String>) -> Output {
        Output {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    fn err(stderr: impl Into<String>) -> Output {
        Output {
            status: 1,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    async fn provider_with(
        process: FakeProcessRunner,
        fs: InMemoryFs,
    ) -> (AndroidProvider, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Registry::open(dir.path()).await.expect("open registry");
        // A real cmdline-tools install so `sdk_root()` resolves to the app-managed dir.
        fs.write_atomic(
            Path::new("/data/sdk/cmdline-tools/latest/bin/sdkmanager"),
            b"#!/bin/sh",
        )
        .await
        .unwrap();
        let provider = AndroidProvider::new(
            Arc::new(process),
            Arc::new(fs),
            registry,
            PathBuf::from("/data"),
            HostOs::Linux,
        );
        (provider, dir)
    }

    fn coord() -> ImageCoord {
        ImageCoord::new(34, ImageType::Default, Abi::X86_64)
    }

    #[tokio::test]
    async fn ensure_image_is_a_no_op_when_already_installed() {
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/data/sdk/system-images/android-34/default/x86_64/source.properties"),
            b"Pkg.Revision=4",
        )
        .await
        .unwrap();
        // No rules registered on the process runner: any call fails the test loudly.
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;

        let (job, log) = JobHandle::collector(emu_core::model::job::JobId("j".into()));
        provider.ensure_image(coord(), &job).await.expect("no-op");
        assert!(log.lock().unwrap().iter().any(|p| p.log_line.as_deref()
            == Some("system-images;android-34;default;x86_64 already installed")));
    }

    /// `FakeProcessRunner` has no real filesystem side effects, so a scripted "success" never
    /// actually unpacks anything — this proves `ensure_image` notices that gap (the safety-net
    /// check after a successful `sdkmanager` run) instead of silently reporting success for a
    /// package that was never really installed. The real, actually-unpacking path is the manual
    /// end-to-end capture in the task file's Notes; a genuine `sdkmanager` process is out of scope
    /// for a unit test (no network, per `AGENTS.md` §6.4).
    #[tokio::test]
    async fn ensure_image_treats_a_success_with_no_marker_as_an_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run("sdkmanager system-images;android-34;default;x86_64", ok(""));
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider
            .ensure_image(
                coord(),
                &JobHandle::noop(emu_core::model::job::JobId("j".into())),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid");
        assert!(err.to_string().contains("not present under"));
    }

    #[tokio::test]
    async fn ensure_image_surfaces_a_real_process_failure() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "sdkmanager system-images;android-34;default;x86_64",
            err("Error: no internet connection"),
        );
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider
            .ensure_image(
                coord(),
                &JobHandle::noop(emu_core::model::job::JobId("j".into())),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "process_failed");
    }

    fn spec() -> CreateSpec {
        CreateSpec {
            avd_name: "pixel6_api34".into(),
            display_name: "Pixel 6 · API 34".into(),
            device_profile_id: "pixel_6".into(),
            image_coord: coord(),
            hardware: emu_core::model::emulator::Hardware::default(),
        }
    }

    #[tokio::test]
    async fn create_runs_avdmanager_and_registers_the_row() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "avdmanager create avd -n pixel6_api34 -k system-images;android-34;default;x86_64 -d pixel_6",
            ok(""),
        );
        let (provider, dir) = provider_with(process, fs).await;

        let id = provider.create(spec()).await.expect("create");
        assert_eq!(id.as_str().len(), 26);

        // Re-open the same on-disk registry and confirm the row really landed.
        let reopened = Registry::open(dir.path()).await.expect("reopen");
        let row = reopened
            .get_emulator(id.as_str())
            .await
            .expect("query")
            .expect("row exists");
        assert_eq!(
            row,
            ("pixel6_api34".to_string(), "Pixel 6 · API 34".to_string())
        );
    }

    #[tokio::test]
    async fn create_maps_duplicate_name_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "avdmanager create avd",
            err("Error: Android Virtual Device 'pixel6_api34' already exists.\nUse --force if you want to replace it.\nnull"),
        );
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider.create(spec()).await.unwrap_err();
        assert_eq!(err.code(), "invalid");
        assert!(err.to_string().contains("already exists locally"));
    }

    #[tokio::test]
    async fn create_maps_unknown_device_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "avdmanager create avd",
            err("Error: No device found matching --device pixel_6.\nnull"),
        );
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider.create(spec()).await.unwrap_err();
        assert_eq!(err.code(), "not_found");
        assert!(err.to_string().contains("pixel_6"));
    }

    #[tokio::test]
    async fn create_maps_invalid_package_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "avdmanager create avd",
            err("Error: Package path is not valid. Valid system image paths are:\nnull"),
        );
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider.create(spec()).await.unwrap_err();
        assert_eq!(err.code(), "not_found");
        assert!(err
            .to_string()
            .contains("system-images;android-34;default;x86_64"));
    }

    #[test]
    fn image_dir_mirrors_the_package_path() {
        assert_eq!(
            image_dir(Path::new("/sdk"), coord()),
            Path::new("/sdk/system-images/android-34/default/x86_64")
        );
    }
}
