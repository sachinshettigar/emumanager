//! [`AndroidProvider`] — the first real [`Provider`] implementation.
//!
//! `ensure_image`/`create` land in task `0015`; `launch`/`stop` in task `0016`; `list_devices`/
//! `list_images` (wiring task `0014`'s parsers to a real installed SDK) in task `0017`; `delete`/
//! `reconcile` are full M3 work (registry reconciliation). Until its own task, each stubs out with
//! [`CoreError::NotImplemented`] rather than a fake implementation.
//!
//! Design choice for `ensure_image` (see task `0015`'s Notes for the real captures behind it):
//! system images are installed by asking the real `sdkmanager` to fetch and unpack them, the exact
//! same "don't reimplement what the real tool already does correctly" choice
//! `crates/emu-core/src/toolchain/bootstrap.rs` made for `platform-tools`/`emulator`. No
//! `Downloader`/zip-extraction code lives here at all.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, timeout, Instant};

use emu_core::error::{CoreError, Result};
use emu_core::model::component::{ComponentId, HostOs};
use emu_core::model::device::DeviceProfile;
use emu_core::model::emulator::{EmulatorId, EmulatorSource, Graphics, LiveState, RunState};
use emu_core::model::image::{ImageCoord, ImageFilter, SystemImage};
use emu_core::model::job::{JobHandle, Progress};
use emu_core::model::plan::CreateSpec;
use emu_core::ports::{ChildProcess, Command, Fs, ProcessRunner};
use emu_core::provider::{LaunchOpts, Provider, RunningHandle};
use emu_core::registry::{EmulatorRow, Registry};
use emu_core::toolchain;
use time::OffsetDateTime;

/// Feed this many `y\n` lines as stdin to a `sdkmanager`/`avdmanager` invocation that might hit a
/// license or confirmation prompt — bounded, not an infinite `yes |`, same rationale and count as
/// `crates/emu-core/src/toolchain/bootstrap.rs::LICENSE_ACCEPT_COUNT` (a real capture never needed
/// more than 7).
const PROMPT_ANSWER_COUNT: usize = 50;

/// How long [`AndroidProvider::launch`] waits for `sys.boot_completed=1` before giving up.
const DEFAULT_BOOT_TIMEOUT: Duration = Duration::from_secs(300);
/// Default for how long [`AndroidProvider::stop`] waits for the emulator to disappear from
/// `adb devices` after a graceful `adb emu kill` before force-killing the child process.
const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(15);
/// Gap between `adb ... getprop sys.boot_completed` polls during a launch, and between
/// "is it gone yet?" checks during a stop.
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// How long a launch spends draining one slice of emulator log output before re-checking boot
/// state — keeps the poll responsive even while the emulator is chatty.
const LOG_DRAIN_SLICE: Duration = Duration::from_millis(250);

/// A live child process the provider is responsible for — shared so a background drain can keep
/// reading its output while `stop` can still `wait`/`kill` it.
type ChildHandle = Arc<AsyncMutex<Box<dyn ChildProcess>>>;

/// One registry-tracked emulator plus its live run state — [`AndroidProvider::tracked_states`]'s
/// element type, what the Dashboard renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedEmulator {
    /// Our stable id.
    pub id: EmulatorId,
    /// The on-disk AVD name.
    pub avd_name: String,
    /// Human-friendly name.
    pub display_name: String,
    /// Current lifecycle state, derived from `adb`.
    pub state: RunState,
    /// `emulator-NNNN` serial when running/booting.
    pub adb_serial: Option<String>,
}

/// Drives `sdkmanager`/`avdmanager`/`emulator`/`adb` via the injected [`ProcessRunner`].
pub struct AndroidProvider {
    process: Arc<dyn ProcessRunner>,
    fs: Arc<dyn Fs>,
    registry: Registry,
    /// The app's data dir (not the SDK dir itself — `<data_dir>/sdk` is the app-managed root;
    /// see `crates/emu-core/src/toolchain/bootstrap.rs`).
    data_dir: PathBuf,
    os: HostOs,
    boot_timeout: Duration,
    stop_timeout: Duration,
    /// Emulator child processes this provider spawned and hasn't stopped yet, keyed by our id.
    /// Holding the handle (rather than dropping it after boot) is what lets `stop` reap the
    /// process — "never orphans the process" in the task's acceptance criteria — and keeps its
    /// stdout pipe drained so a chatty emulator never blocks on a full buffer.
    running: StdMutex<HashMap<EmulatorId, ChildHandle>>,
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
            boot_timeout: DEFAULT_BOOT_TIMEOUT,
            stop_timeout: DEFAULT_STOP_TIMEOUT,
            running: StdMutex::new(HashMap::new()),
        }
    }

    /// Override the boot-complete timeout (default [`DEFAULT_BOOT_TIMEOUT`]). For tests and for a
    /// caller that knows its host is slow.
    #[must_use]
    pub fn with_boot_timeout(mut self, timeout: Duration) -> Self {
        self.boot_timeout = timeout;
        self
    }

    /// Override how long `stop` waits after `adb emu kill` before force-killing (default
    /// [`DEFAULT_STOP_TIMEOUT`]).
    #[must_use]
    pub fn with_stop_timeout(mut self, timeout: Duration) -> Self {
        self.stop_timeout = timeout;
        self
    }

    /// The SDK root to run `sdkmanager`/`avdmanager`/`emulator`/`adb` under: wherever
    /// `cmdline-tools` was actually found (app-managed, or a real system SDK — task `0012`'s
    /// "reuse what's already there" rule), falling back to the app-managed dir if nothing has been
    /// scanned as installed yet (a real invocation would then fail with a clear "no such file"
    /// `Process` error, which is honest — there is truly nothing to run).
    ///
    /// # Errors
    /// [`CoreError`] from the filesystem scan.
    pub async fn sdk_root(&self) -> Result<PathBuf> {
        let app_sdk_dir = self.data_dir.join("sdk");
        let state = toolchain::scan(self.fs.as_ref(), &app_sdk_dir, self.os, |k| {
            std::env::var(k).ok()
        })
        .await?;
        Ok(state
            .location_of(ComponentId::CmdlineTools)
            .map_or(app_sdk_dir, |l| l.sdk_root.clone()))
    }

    /// `true` when `coord`'s system image is installed under the resolved SDK root — the same
    /// `source.properties` marker check `ensure_image` uses.
    ///
    /// # Errors
    /// [`CoreError`] from the filesystem port.
    pub async fn is_image_installed(&self, coord: ImageCoord) -> Result<bool> {
        let sdk_root = self.sdk_root().await?;
        self.fs.exists(&image_marker_file(&sdk_root, coord)).await
    }

    /// Every registry-tracked emulator with its live [`RunState`], probed from `adb`. A
    /// lightweight, read-only view for the Dashboard — **not** `reconcile()`: it never adopts
    /// out-of-band AVDs or drops vanished rows (that is M3's milestone), only reports the current
    /// state of what the registry already knows.
    ///
    /// # Errors
    /// [`CoreError`] from the registry or, for the initial device list, the process port. A
    /// per-emulator `adb` hiccup is swallowed (that emulator reads as `Stopped`).
    pub async fn tracked_states(&self) -> Result<Vec<TrackedEmulator>> {
        let rows = self.registry.list_rows().await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);
        let serials = self.emulator_serials(&adb).await.unwrap_or_default();

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut adb_serial = None;
            for candidate in &serials {
                let cmd = Command::new(adb.display().to_string())
                    .args(["-s", candidate])
                    .args(["emu", "avd", "name"]);
                if let Ok(out) = self.process.run(cmd).await {
                    if out.stdout.lines().next().unwrap_or("").trim() == row.avd_name {
                        adb_serial = Some(candidate.clone());
                        break;
                    }
                }
            }
            let state = match &adb_serial {
                None => RunState::Stopped,
                Some(serial) if self.boot_completed(&adb, serial).await => RunState::Running,
                Some(_) => RunState::Booting,
            };
            out.push(TrackedEmulator {
                id: row.id,
                avd_name: row.avd_name,
                display_name: row.display_name,
                state,
                adb_serial,
            });
        }
        Ok(out)
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

    fn adb_path(&self, sdk_root: &Path) -> PathBuf {
        toolchain::binary_path(sdk_root, ComponentId::PlatformTools, self.os)
    }

    fn emulator_path(&self, sdk_root: &Path) -> PathBuf {
        toolchain::binary_path(sdk_root, ComponentId::Emulator, self.os)
    }

    fn prompt_answer_stdin() -> Vec<u8> {
        "y\n".repeat(PROMPT_ANSWER_COUNT).into_bytes()
    }

    /// Serials of every emulator currently in `adb devices` (state ignored — an entry that isn't
    /// `device` yet still becomes one shortly). Real output shape (cited, Android platform-tools
    /// `adb devices`):
    ///
    /// ```text
    /// List of devices attached
    /// emulator-5554\tdevice
    /// ```
    async fn emulator_serials(&self, adb: &Path) -> Result<Vec<String>> {
        let out = self
            .process
            .run(Command::new(adb.display().to_string()).arg("devices"))
            .await?;
        Ok(out
            .stdout
            .lines()
            .skip(1) // "List of devices attached"
            .filter_map(|line| line.split_whitespace().next())
            .filter(|serial| serial.starts_with("emulator-"))
            .map(str::to_string)
            .collect())
    }

    /// `true` once `adb -s <serial> shell getprop sys.boot_completed` prints `1`. An adb error
    /// (device still `offline`, console not up yet) counts as "not booted" — the caller keeps
    /// polling until its own deadline.
    async fn boot_completed(&self, adb: &Path, serial: &str) -> bool {
        let cmd = Command::new(adb.display().to_string())
            .args(["-s", serial])
            .args(["shell", "getprop", "sys.boot_completed"]);
        matches!(self.process.run(cmd).await, Ok(out) if out.stdout.trim() == "1")
    }

    /// The `emulator-NNNN` serial whose running AVD is `avd_name`, if one is up. Uses the emulator
    /// console `avd name` command (`adb -s <serial> emu avd name`), whose reply is the AVD name on
    /// its own line followed by `OK` (cited: Android emulator console reference).
    async fn serial_for_avd(&self, adb: &Path, avd_name: &str) -> Result<Option<String>> {
        for serial in self.emulator_serials(adb).await? {
            let cmd = Command::new(adb.display().to_string())
                .args(["-s", &serial])
                .args(["emu", "avd", "name"]);
            if let Ok(out) = self.process.run(cmd).await {
                let reported = out.stdout.lines().next().unwrap_or("").trim();
                if reported == avd_name {
                    return Ok(Some(serial));
                }
            }
        }
        Ok(None)
    }

    /// Best-effort "is `avd_name` still running?" for the `stop` wait loop: an adb error (device
    /// mid-shutdown, console gone) is treated as "assume still there, keep waiting" rather than
    /// "gone", so a transient failure can't make `stop` declare success early.
    async fn avd_still_running(&self, adb: &Path, avd_name: &str) -> bool {
        !matches!(self.serial_for_avd(adb, avd_name).await, Ok(None))
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
/// not an invented marker (confirmed present in the real system-image archive task `0015`
/// downloaded and inspected — see that task file's Notes).
fn image_marker_file(sdk_root: &Path, coord: ImageCoord) -> PathBuf {
    image_dir(sdk_root, coord).join("source.properties")
}

/// The `-gpu <mode>` value for a [`Graphics`] choice. Values cited from the official emulator
/// command-line reference (<https://developer.android.com/studio/run/emulator-commandline>). A
/// future `Graphics` variant this doesn't know yet falls back to the emulator's own default.
fn gpu_mode(graphics: Graphics) -> &'static str {
    match graphics {
        Graphics::Host => "host",
        Graphics::SwiftshaderIndirect => "swiftshader_indirect",
        // `Graphics::Auto` and any future non_exhaustive variant → the emulator's own default.
        _ => "auto",
    }
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
    /// list came from the real "unknown flag" usage dump — see task `0015`'s Notes). Always
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
        let row = EmulatorRow::new(
            id.clone(),
            spec.avd_name.clone(),
            spec.display_name.clone(),
            spec.device_profile_id.clone(),
            spec.image_coord,
            spec.hardware.clone(),
            EmulatorSource::Manual { discovered: false },
            OffsetDateTime::now_utc(),
        );
        self.registry.upsert_emulator(&row).await?;
        Ok(id)
    }

    /// Spawn `emulator @<avd_name>` (+ flags from `opts`), stream its log lines onto `job`, and
    /// poll `adb` until `sys.boot_completed=1` or [`Self::boot_timeout`] elapses. On success the
    /// child handle is kept in [`Self::running`] so `stop` can reap it. Flags are cited from the
    /// official emulator command-line reference
    /// (<https://developer.android.com/studio/run/emulator-commandline>).
    async fn launch(
        &self,
        id: EmulatorId,
        opts: LaunchOpts,
        job: &JobHandle,
    ) -> Result<RunningHandle> {
        let avd_name = self
            .registry
            .get_row(id.as_str())
            .await?
            .ok_or_else(|| CoreError::NotFound {
                what: "emulator",
                name: id.to_string(),
            })?
            .avd_name;

        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);

        let mut cmd = Command::new(self.emulator_path(&sdk_root).display().to_string())
            .arg(format!("@{avd_name}"));
        if opts.headless {
            cmd = cmd.arg("-no-window");
        }
        if opts.wipe_data {
            cmd = cmd.arg("-wipe-data");
        }
        if opts.cold_boot {
            cmd = cmd.arg("-no-snapshot-load");
        }
        if let Some(graphics) = opts.graphics {
            cmd = cmd.args(["-gpu", gpu_mode(graphics)]);
        }
        cmd = cmd.args(opts.extra_args.iter().cloned());

        job.report(Progress::log(format!("launching {}", cmd.display())));
        let child = self.process.spawn(cmd).await?;
        let pid = child.pid();
        let child: ChildHandle = Arc::new(AsyncMutex::new(child));

        let serial = match self.wait_for_boot(&adb, &avd_name, &child, job).await {
            Ok(serial) => serial,
            Err(e) => {
                // Boot failed — don't leave the child we spawned unreaped.
                let mut guard = child.lock().await;
                let _ = guard.kill().await;
                let _ = guard.wait().await;
                return Err(e);
            }
        };

        self.running
            .lock()
            .expect("running map poisoned")
            .insert(id.clone(), Arc::clone(&child));

        job.report(Progress::log(format!("{avd_name} booted ({serial})")));
        Ok(RunningHandle {
            id,
            adb_serial: Some(serial),
            pid,
            // The emulator's gRPC control port isn't parsed from its log yet — M3's detail panel
            // (`docs/spec.md` §5.3) is where that (and a live post-boot log stream) belong.
            grpc_port: None,
        })
    }

    /// Graceful `adb -s <serial> emu kill` first; if the emulator is still listed after
    /// [`Self::stop_timeout`], fall back to killing the child process this provider spawned. Reaps
    /// the child handle either way. Idempotent: a `stop` for an AVD that isn't running succeeds.
    async fn stop(&self, id: EmulatorId) -> Result<()> {
        let avd_name = self
            .registry
            .get_row(id.as_str())
            .await?
            .ok_or_else(|| CoreError::NotFound {
                what: "emulator",
                name: id.to_string(),
            })?
            .avd_name;

        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);

        let child = self
            .running
            .lock()
            .expect("running map poisoned")
            .remove(&id);

        let Some(serial) = self.serial_for_avd(&adb, &avd_name).await? else {
            // Nothing running for this AVD. Still reap any child we were holding, then succeed.
            if let Some(child) = child {
                let _ = child.lock().await.wait().await;
            }
            return Ok(());
        };

        // Graceful shutdown via the emulator console.
        let kill = Command::new(adb.display().to_string())
            .args(["-s", &serial])
            .args(["emu", "kill"]);
        let out = self.process.run(kill).await?;
        if !out.success() {
            return Err(CoreError::Process {
                program: "adb emu kill".to_string(),
                code: out.status,
                stderr: out.stderr,
            });
        }

        // Wait for it to actually disappear from `adb devices`.
        let deadline = Instant::now() + self.stop_timeout;
        while self.avd_still_running(&adb, &avd_name).await {
            if Instant::now() >= deadline {
                // Still there — force-kill the child we spawned, if we're holding it.
                if let Some(child) = child {
                    let mut guard = child.lock().await;
                    guard.kill().await?;
                    let _ = guard.wait().await;
                    return Ok(());
                }
                return Err(CoreError::Process {
                    program: "adb emu kill".to_string(),
                    code: -1,
                    stderr: format!(
                        "emulator '{avd_name}' ({serial}) did not exit within {:?} and this \
                         process has no handle to it (started out-of-band, or the app was \
                         restarted) — kill it manually; pid-tracked force-stop lands with M3's \
                         registry",
                        self.stop_timeout
                    ),
                });
            }
            sleep(POLL_INTERVAL).await;
        }

        // Gone cleanly. Reap any child we were holding.
        if let Some(child) = child {
            let _ = child.lock().await.wait().await;
        }
        Ok(())
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

impl AndroidProvider {
    /// Stream emulator log lines onto `job` while polling `adb` for `sys.boot_completed=1`.
    /// Returns the discovered `emulator-NNNN` serial on success.
    ///
    /// Fails fast with a [`CoreError::Process`] if the emulator's output stream closes (the
    /// process exited) before boot; otherwise fails with a timeout error once
    /// [`Self::boot_timeout`] elapses. The pure wall-clock-deadline branch (stream still open,
    /// deadline hit) is exercised by real use, not the fake-driven unit tests — `FakeChild` always
    /// reaches EOF (see task `0016`'s Notes).
    async fn wait_for_boot(
        &self,
        adb: &Path,
        avd_name: &str,
        child: &ChildHandle,
        job: &JobHandle,
    ) -> Result<String> {
        let deadline = Instant::now() + self.boot_timeout;
        let mut serial: Option<String> = None;
        let mut last_poll = Instant::now()
            .checked_sub(POLL_INTERVAL)
            .unwrap_or_else(Instant::now); // force an immediate first poll

        loop {
            if Instant::now() >= deadline {
                return Err(CoreError::Process {
                    program: "emulator".to_string(),
                    code: -1,
                    stderr: format!(
                        "'{avd_name}' did not reach sys.boot_completed=1 within {:?}",
                        self.boot_timeout
                    ),
                });
            }

            if last_poll.elapsed() >= POLL_INTERVAL {
                last_poll = Instant::now();
                if serial.is_none() {
                    serial = self.emulator_serials(adb).await?.into_iter().next();
                }
                if let Some(s) = &serial {
                    if self.boot_completed(adb, s).await {
                        return Ok(s.clone());
                    }
                }
            }

            let mut guard = child.lock().await;
            match timeout(LOG_DRAIN_SLICE, guard.next_line()).await {
                Ok(Ok(Some(line))) => job.report(Progress::log(line)),
                Ok(Ok(None)) => {
                    // The emulator's output stream closed — the process has exited. One last boot
                    // check in case it handed off, then fail fast rather than waiting out the
                    // whole timeout.
                    drop(guard);
                    if let Some(s) = &serial {
                        if self.boot_completed(adb, s).await {
                            return Ok(s.clone());
                        }
                    }
                    let status = child.lock().await.wait().await.map_or(-1, |o| o.status);
                    return Err(CoreError::Process {
                        program: "emulator".to_string(),
                        code: status,
                        stderr: format!(
                            "emulator exited (code {status}) before sys.boot_completed=1"
                        ),
                    });
                }
                Ok(Err(e)) => return Err(e),
                Err(_elapsed) => {} // no line this slice — loop, re-check deadline / boot state
            }
        }
    }
}

/// Turn a real `avdmanager create avd` failure into a specific, actionable [`CoreError`]. All
/// three matched substrings are real, captured `stderr` text (task `0015`'s Notes); anything else
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
    use emu_core::model::job::JobId;
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

    fn job() -> JobHandle {
        JobHandle::noop(JobId("j".into()))
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

    fn spec() -> CreateSpec {
        CreateSpec {
            avd_name: "pixel6_api34".into(),
            display_name: "Pixel 6 · API 34".into(),
            device_profile_id: "pixel_6".into(),
            image_coord: coord(),
            hardware: emu_core::model::emulator::Hardware::default(),
        }
    }

    // ---- ensure_image / create (task 0015) --------------------------------------------------

    #[tokio::test]
    async fn ensure_image_is_a_no_op_when_already_installed() {
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/data/sdk/system-images/android-34/default/x86_64/source.properties"),
            b"Pkg.Revision=4",
        )
        .await
        .unwrap();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;

        let (job, log) = JobHandle::collector(JobId("j".into()));
        provider.ensure_image(coord(), &job).await.expect("no-op");
        assert!(log.lock().unwrap().iter().any(|p| p.log_line.as_deref()
            == Some("system-images;android-34;default;x86_64 already installed")));
    }

    /// `FakeProcessRunner` has no real filesystem side effects, so a scripted "success" never
    /// actually unpacks anything — this proves `ensure_image` notices that gap (the safety-net
    /// check after a successful `sdkmanager` run) instead of silently reporting success for a
    /// package that was never really installed.
    #[tokio::test]
    async fn ensure_image_treats_a_success_with_no_marker_as_an_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run("sdkmanager system-images;android-34;default;x86_64", ok(""));
        let (provider, _dir) = provider_with(process, fs).await;

        let err = provider.ensure_image(coord(), &job()).await.unwrap_err();
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

        let err = provider.ensure_image(coord(), &job()).await.unwrap_err();
        assert_eq!(err.code(), "process_failed");
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

        let reopened = Registry::open(dir.path()).await.expect("reopen");
        let row = reopened
            .get_row(id.as_str())
            .await
            .expect("query")
            .expect("row exists");
        assert_eq!(row.avd_name, "pixel6_api34");
        assert_eq!(row.display_name, "Pixel 6 · API 34");
        assert_eq!(row.device_profile_id, "pixel_6");
        assert_eq!(row.image_coord, Some(coord()));
        assert_eq!(row.last_state, RunState::Stopped);
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

    #[test]
    fn gpu_mode_values_match_the_docs() {
        assert_eq!(gpu_mode(Graphics::Auto), "auto");
        assert_eq!(gpu_mode(Graphics::Host), "host");
        assert_eq!(
            gpu_mode(Graphics::SwiftshaderIndirect),
            "swiftshader_indirect"
        );
    }

    // ---- launch / stop (task 0016) --------------------------------------------------------

    /// Register a created emulator's row and return its id.
    async fn seed_emulator(provider: &AndroidProvider) -> EmulatorId {
        let id = EmulatorId::generate();
        let row = EmulatorRow::new(
            id.clone(),
            "pixel6_api34".into(),
            "Pixel 6 · API 34".into(),
            "pixel_6".into(),
            coord(),
            emu_core::model::emulator::Hardware::default(),
            EmulatorSource::Manual { discovered: false },
            OffsetDateTime::now_utc(),
        );
        provider
            .registry
            .upsert_emulator(&row)
            .await
            .expect("seed row");
        id
    }

    #[tokio::test]
    async fn launch_streams_logs_then_returns_a_running_handle_on_boot() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_spawn(
                "emulator @pixel6_api34",
                [
                    "emulator: Android emulator version 35.1",
                    "emulator: boot started",
                ],
                ok(""),
            )
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("0\n"))
            .on_run("shell getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        let (job, log) = JobHandle::collector(JobId("j".into()));
        let handle = provider
            .launch(id.clone(), LaunchOpts::default(), &job)
            .await
            .expect("launch");

        assert_eq!(handle.adb_serial.as_deref(), Some("emulator-5554"));
        assert_eq!(handle.id, id);
        let lines: Vec<_> = log
            .lock()
            .unwrap()
            .iter()
            .filter_map(|p| p.log_line.clone())
            .collect();
        assert!(lines.iter().any(|l| l.contains("boot started")));
        assert!(lines.iter().any(|l| l.contains("launching")));
    }

    #[tokio::test]
    async fn launch_passes_headless_and_gpu_flags_from_opts() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_spawn(
                "emulator @pixel6_api34 -no-window -gpu swiftshader_indirect",
                Vec::<String>::new(),
                ok(""),
            )
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        let opts = LaunchOpts {
            headless: true,
            graphics: Some(Graphics::SwiftshaderIndirect),
            ..LaunchOpts::default()
        };
        provider
            .launch(id, opts, &job())
            .await
            .expect("launch with flags");
        // The `on_spawn` needle above only matches if the argv was built with those exact flags.
    }

    #[tokio::test]
    async fn launch_fails_fast_when_the_emulator_exits_before_boot() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_spawn(
                "emulator @pixel6_api34",
                ["emulator: PANIC: Missing emulator engine program"],
                Output {
                    status: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                },
            )
            .on_run("adb devices", ok("List of devices attached\n"))
            .on_run("shell getprop sys.boot_completed", ok("0\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        let err = provider
            .launch(id, LaunchOpts::default(), &job())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "process_failed");
        assert!(err.to_string().contains("before sys.boot_completed"));
    }

    #[tokio::test]
    async fn launch_times_out_when_boot_never_completes() {
        let fs = InMemoryFs::new();
        // The emulator process lingers (stream stays open, never EOFs) but never boots: `adb
        // devices` answers once, and every `getprop` says "not booted" (the first scripted, the
        // rest unscripted → treated as not-booted). The short boot timeout then fires. Real time,
        // but a 1 ms timeout so the whole test is over in one `LOG_DRAIN_SLICE`.
        let process = FakeProcessRunner::new()
            .on_spawn_lingering("emulator @pixel6_api34", ["emulator: warming up"], ok(""))
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("0\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let provider = provider.with_boot_timeout(Duration::from_millis(1));
        let id = seed_emulator(&provider).await;

        let err = provider
            .launch(id, LaunchOpts::default(), &job())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "process_failed");
        assert!(err.to_string().contains("did not reach sys.boot_completed"));
    }

    #[tokio::test]
    async fn stop_force_kills_when_graceful_shutdown_is_ignored() {
        let fs = InMemoryFs::new();
        // Launch succeeds (child stored in `running`); then the emulator ignores `emu kill` and
        // stays in `adb devices`, so `stop` force-kills the child after its (1 ms) stop timeout.
        let process = FakeProcessRunner::new()
            .on_spawn_lingering("emulator @pixel6_api34", ["emulator: booted"], ok(""))
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("1\n"))
            // stop: serial_for_avd
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("emu avd name", ok("pixel6_api34\nOK\n"))
            .on_run("emu kill", ok("OK: killing emulator, bye bye\n"))
            // stop's wait loop: still there once, then unscripted calls read as "still there"
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("emu avd name", ok("pixel6_api34\nOK\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let provider = provider.with_stop_timeout(Duration::ZERO);
        let id = seed_emulator(&provider).await;

        provider
            .launch(id.clone(), LaunchOpts::default(), &job())
            .await
            .expect("launch");
        provider
            .stop(id)
            .await
            .expect("force-kill stop still succeeds");
    }

    #[tokio::test]
    async fn stop_sends_emu_kill_to_the_matching_serial() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            // serial_for_avd: list devices, then ask each for its avd name
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("emu avd name", ok("pixel6_api34\nOK\n"))
            .on_run(
                "-s emulator-5554 emu kill",
                ok("OK: killing emulator, bye bye\n"),
            )
            // the post-kill "is it gone?" check: no devices left
            .on_run("adb devices", ok("List of devices attached\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        provider.stop(id).await.expect("stop");
    }

    #[tokio::test]
    async fn stop_is_idempotent_when_nothing_is_running() {
        let fs = InMemoryFs::new();
        let process =
            FakeProcessRunner::new().on_run("adb devices", ok("List of devices attached\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        provider.stop(id).await.expect("idempotent stop");
    }

    #[tokio::test]
    async fn stop_of_an_unknown_id_is_not_found() {
        let fs = InMemoryFs::new();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;

        let err = provider.stop(EmulatorId::generate()).await.unwrap_err();
        assert_eq!(err.code(), "not_found");
    }
}
