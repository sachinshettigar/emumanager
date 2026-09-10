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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, timeout, Instant};

use crate::avd_list::{parse_avdmanager_list_avd, AvdEntry};

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
pub type ChildHandle = Arc<AsyncMutex<Box<dyn ChildProcess>>>;

/// A quick read of a running device: what the inspector's facts strip shows. Every field is
/// `Option` — a value only appears when its `adb` one-shot ran and parsed cleanly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceFacts {
    /// `ro.product.model`.
    pub model: Option<String>,
    /// `ro.build.version.release` (e.g. `"14"`).
    pub android_release: Option<String>,
    /// `ro.build.version.sdk` (API level).
    pub sdk_int: Option<u32>,
    /// Battery charge percent from `dumpsys battery` (`level:`).
    pub battery_pct: Option<u8>,
    /// Free space on `/data`, MB, from `df /data`.
    pub data_free_mb: Option<u64>,
    /// Total size of `/data`, MB, from `df /data`.
    pub data_total_mb: Option<u64>,
}

/// Live network state of a running device — [`AndroidProvider::device_network`]'s return.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceNetwork {
    pub interfaces: Vec<NetInterface>,
    pub connections: Vec<NetConnection>,
}

/// One address on one network interface (from `ip -o addr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetInterface {
    /// e.g. `wlan0`, `radio0`, `lo`.
    pub name: String,
    /// e.g. `10.0.2.16/24` (family kept as printed: `inet` / `inet6`).
    pub addr: String,
}

/// One open IPv4 socket (from `/proc/net/tcp` or `/proc/net/udp`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConnection {
    /// `"tcp"` or `"udp"`.
    pub proto: &'static str,
    /// `ip:port`.
    pub local: String,
    /// `ip:port`; `0.0.0.0:0` for a listener / unconnected UDP.
    pub remote: String,
    /// TCP state (`ESTABLISHED`, `LISTEN`, …); empty for UDP.
    pub state: &'static str,
    /// Owning Linux uid.
    pub uid: u32,
    /// Package name for `uid`, when `pm list packages -U` mapped it.
    pub package: Option<String>,
}

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
    /// Long-lived `adb logcat` streams the device inspector started, keyed by our id. One per
    /// emulator; starting a new one replaces (and kills) any previous. Reaped on `shutdown`.
    logcats: StdMutex<HashMap<EmulatorId, ChildHandle>>,
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
            logcats: StdMutex::new(HashMap::new()),
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
        Ok(self
            .installed_state()
            .await?
            .location_of(ComponentId::CmdlineTools)
            .map_or(app_sdk_dir, |l| l.sdk_root.clone()))
    }

    /// Scan what SDK components are installed (app-managed dir + any system SDK). Feeds
    /// `emu_core::profile::resolve` (task `0023`'s `inspect_profile`).
    ///
    /// # Errors
    /// [`CoreError`] from the filesystem scan.
    pub async fn installed_state(&self) -> Result<emu_core::toolchain::InstalledState> {
        toolchain::scan(self.fs.as_ref(), &self.data_dir.join("sdk"), self.os, |k| {
            std::env::var(k).ok()
        })
        .await
    }

    /// The registry handle — for the profile commands that read/write saved recipes.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Every entry `avdmanager list avd` reports (loadable *or* broken), including the on-disk
    /// `.avd` path when it prints one. `[]` if `avdmanager` can't run.
    async fn list_avd_entries(&self) -> Vec<AvdEntry> {
        let Ok(sdk_root) = self.sdk_root().await else {
            return Vec::new();
        };
        let avdmanager = self.avdmanager_path(&sdk_root);
        match self
            .process
            .run(Command::new(avdmanager.display().to_string()).args(["list", "avd"]))
            .await
        {
            Ok(out) => parse_avdmanager_list_avd(&out.stdout),
            Err(_) => Vec::new(),
        }
    }

    /// Every AVD name `avdmanager list avd` reports (loadable *or* broken) — the set an
    /// `avdmanager create avd -n <name>` would collide with. `Ok([])` if `avdmanager` can't run.
    async fn list_avd_names(&self) -> Vec<String> {
        self.list_avd_entries()
            .await
            .into_iter()
            .map(|a| a.name)
            .collect()
    }

    /// Make sure the AVD at `avd_dir` boots with `hw.keyboard=yes` so the **desktop** keyboard
    /// types into Android text fields. `avdmanager create avd` leaves the key out of `config.ini`
    /// and the emulator then defaults a phone AVD to `hw.keyboard=no` — only the on-screen
    /// keyboard works and host keystrokes are dropped. Android Studio's AVD Manager writes `yes`;
    /// we match it. (`hw.keyboard` is a documented AVD hardware property carried in the AVD's
    /// `config.ini`; see the emulator command-line reference,
    /// <https://developer.android.com/studio/run/emulator-commandline>.)
    ///
    /// Look up `avd_name`'s on-disk dir via `avdmanager list avd` and run [`Self::ensure_hw_keyboard`]
    /// on it. Returns the launch-log line, or `None` if the AVD dir can't be found or nothing
    /// needed changing.
    async fn ensure_hw_keyboard_for(&self, avd_name: &str) -> Option<String> {
        let avd_dir = self
            .list_avd_entries()
            .await
            .into_iter()
            .find(|e| e.name == avd_name)
            .and_then(|e| e.path)?;
        self.ensure_hw_keyboard(&avd_dir).await
    }

    /// Returns a line for the launch log when it changed something (or tried and failed), else
    /// `None`. Best-effort: a missing/unreadable/unwritable `config.ini` never blocks a launch.
    async fn ensure_hw_keyboard(&self, avd_dir: &Path) -> Option<String> {
        let config = avd_dir.join("config.ini");
        let bytes = self.fs.read(&config).await.ok()?;
        let text = std::str::from_utf8(&bytes).ok()?;
        let fixed = hw_keyboard_fix(text)?;
        match self.fs.write_atomic(&config, fixed.as_bytes()).await {
            Ok(()) => Some(
                "enabled the hardware keyboard (hw.keyboard=yes) so your desktop keyboard types \
                 into the emulator"
                    .to_string(),
            ),
            Err(e) => Some(format!(
                "could not enable the hardware keyboard in config.ini ({e}); the on-screen \
                 keyboard still works"
            )),
        }
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

    /// The full stored record for `id`. `NotFound` if untracked.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id; [`CoreError`] from the registry.
    pub async fn detail(&self, id: &EmulatorId) -> Result<EmulatorRow> {
        self.registry
            .get_row(id.as_str())
            .await?
            .ok_or_else(|| CoreError::NotFound {
                what: "emulator",
                name: id.to_string(),
            })
    }

    /// Change an emulator's display name. Made unique against the *other* emulators' names
    /// (`Name` → `Name (2)`), so renames can't produce two identical rows. `NotFound` if untracked.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id; [`CoreError`] from the registry.
    pub async fn rename(&self, id: &EmulatorId, display_name: &str) -> Result<()> {
        self.detail(id).await?; // existence check
        let taken: Vec<String> = self
            .registry
            .list_rows()
            .await?
            .into_iter()
            .filter(|r| r.id.as_str() != id.as_str())
            .map(|r| r.display_name)
            .collect();
        let unique = emu_core::profile::unique_display_name(display_name, &taken);
        self.registry.rename(id.as_str(), &unique).await
    }

    /// Replace an emulator's provenance (profile apply → `Imported`). `NotFound` if untracked.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id; [`CoreError`] from the registry.
    pub async fn set_source(&self, id: &EmulatorId, source: &EmulatorSource) -> Result<()> {
        self.detail(id).await?;
        self.registry.set_source(id.as_str(), source).await
    }

    /// Update an emulator's stored hardware config. Takes effect the next time the AVD is
    /// (re)created — this does **not** rewrite the live `config.ini` or recreate the AVD (that is a
    /// later task); it records the user's intent so the detail panel and a future profile export
    /// are correct. `NotFound` if untracked.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id; [`CoreError`] from the registry.
    pub async fn set_hardware(
        &self,
        id: &EmulatorId,
        hardware: &emu_core::model::emulator::Hardware,
    ) -> Result<()> {
        self.detail(id).await?;
        self.registry.set_hardware(id.as_str(), hardware).await
    }

    /// Wipe an emulator's user data by deleting the AVD's writable images and snapshots, so the
    /// next launch rebuilds them from the system image (the same effect as `emulator -wipe-data`).
    /// Refuses while the emulator is running. A file that isn't there is not an error.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id; [`CoreError::Invalid`] if it's running;
    /// [`CoreError`] from the filesystem port.
    pub async fn wipe_data(&self, id: &EmulatorId, avd_dir: &Path) -> Result<()> {
        // The writable state an `-wipe-data` launch would otherwise recreate. Names are the ones
        // the emulator writes into an AVD dir; a missing one just means it was never booted.
        const WIPE: &[&str] = &[
            "userdata-qemu.img",
            "userdata-qemu.img.qcow2",
            "userdata.img",
            "userdata.img.qcow2",
            "cache.img",
            "cache.img.qcow2",
            "snapshots",
        ];

        let row = self.detail(id).await?;
        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);
        if self.serial_for_avd(&adb, &row.avd_name).await?.is_some() {
            return Err(CoreError::invalid(
                "emulator",
                format!(
                    "'{}' is running — stop it before wiping data",
                    row.display_name
                ),
            ));
        }
        for name in WIPE {
            let path = avd_dir.join(name);
            if self.fs.exists(&path).await? {
                self.fs.remove(&path).await?;
            }
        }
        Ok(())
    }

    /// The on-disk launch-log file for `avd_name`: `<data_dir>/logs/<avd_name>.log`. `launch`
    /// truncates it at the start of each run and tees every streamed line into it; the detail
    /// panel tails it (`read_log_tail`).
    #[must_use]
    pub fn log_path(&self, avd_name: &str) -> PathBuf {
        self.data_dir.join("logs").join(format!("{avd_name}.log"))
    }

    /// The last `max_lines` lines of `id`'s launch log, oldest first. Empty (not an error) when the
    /// emulator has never been launched or the log can't be read.
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id.
    pub async fn read_log_tail(&self, id: &EmulatorId, max_lines: usize) -> Result<Vec<String>> {
        let row = self.detail(id).await?;
        let Ok(bytes) = self.fs.read(&self.log_path(&row.avd_name)).await else {
            return Ok(Vec::new());
        };
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        Ok(lines[start..].iter().map(|s| (*s).to_string()).collect())
    }

    /// Report `line` on the job stream *and* append it to the emulator's on-disk log. One
    /// read-modify-write per line — fine for a boot log (a few KB); a streaming `Fs::append` is a
    /// future port addition if it ever matters.
    async fn emit_log(&self, job: &JobHandle, log_path: &Path, line: String) {
        job.report(Progress::log(line.clone()));
        let mut contents = self.fs.read(log_path).await.unwrap_or_default();
        contents.extend_from_slice(line.as_bytes());
        contents.push(b'\n');
        let _ = self.fs.write_atomic(log_path, &contents).await;
    }

    /// Kill and reap every emulator child process this provider still holds. Call on app shutdown
    /// so quitting never orphans an emulator we launched. Best-effort and time-boxed per child.
    pub async fn shutdown(&self) {
        let children: Vec<ChildHandle> = {
            let mut running = self.running.lock().expect("running map poisoned");
            let mut logcats = self.logcats.lock().expect("logcats map poisoned");
            running
                .drain()
                .chain(logcats.drain())
                .map(|(_, child)| child)
                .collect()
        };
        for child in children {
            let mut guard = child.lock().await;
            let _ = timeout(Duration::from_secs(5), async {
                let _ = guard.kill().await;
                let _ = guard.wait().await;
            })
            .await;
        }
    }

    /// Resolve the `emulator-NNNN` serial for a tracked emulator that is currently running, or a
    /// clear error if it isn't.
    async fn running_serial(&self, id: &EmulatorId) -> Result<String> {
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
        self.serial_for_avd(&adb, &avd_name)
            .await?
            .ok_or_else(|| CoreError::Invalid {
                what: "device inspector",
                detail: format!("emulator '{avd_name}' isn't running"),
            })
    }

    /// Start streaming `adb -s <serial> logcat -v threadtime` for a running emulator and hand back
    /// the child handle — the caller drains its lines and emits them. Any existing stream for the
    /// same id is killed first. `-v threadtime` is the documented parseable format
    /// (<https://developer.android.com/tools/logcat#outputFormat>).
    ///
    /// # Errors
    /// [`CoreError::NotFound`] for an unknown id, [`CoreError::Invalid`] when it isn't running.
    pub async fn logcat_start(&self, id: &EmulatorId) -> Result<ChildHandle> {
        let serial = self.running_serial(id).await?;
        self.logcat_stop(id).await;

        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);
        let cmd = Command::new(adb.display().to_string())
            .args(["-s", &serial])
            .args(["logcat", "-v", "threadtime"]);
        let child = self.process.spawn(cmd).await?;
        let handle: ChildHandle = Arc::new(AsyncMutex::new(child));
        self.logcats
            .lock()
            .expect("logcats map poisoned")
            .insert(id.clone(), Arc::clone(&handle));
        Ok(handle)
    }

    /// Kill and forget the `adb logcat` stream for `id`, if any. Idempotent.
    pub async fn logcat_stop(&self, id: &EmulatorId) {
        let handle = self
            .logcats
            .lock()
            .expect("logcats map poisoned")
            .remove(id);
        if let Some(handle) = handle {
            let mut guard = handle.lock().await;
            let _ = guard.kill().await;
            let _ = guard.wait().await;
        }
    }

    /// A quick snapshot of a running emulator: model, Android version, battery, `/data` usage —
    /// each from a small `adb shell` one-shot, parsed defensively (a value that doesn't parse is
    /// simply left `None`).
    ///
    /// `getprop` keys and `df` / `dumpsys battery` output shapes are standard and stable; the
    /// parsers here only take the first plausible token and never guess.
    pub async fn device_facts(&self, id: &EmulatorId) -> Result<DeviceFacts> {
        let serial = self.running_serial(id).await?;
        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);

        let getprop = |key: &str| {
            let adb = adb.display().to_string();
            let serial = serial.clone();
            let key = key.to_string();
            async move {
                self.process
                    .run(
                        Command::new(adb)
                            .args(["-s", &serial])
                            .args(["shell", "getprop", &key]),
                    )
                    .await
                    .ok()
                    .map(|o| o.stdout.trim().to_string())
                    .filter(|s| !s.is_empty())
            }
        };

        let model = getprop("ro.product.model").await;
        let android_release = getprop("ro.build.version.release").await;
        let sdk_int = getprop("ro.build.version.sdk")
            .await
            .and_then(|s| s.parse().ok());

        let battery_pct = self
            .process
            .run(
                Command::new(adb.display().to_string())
                    .args(["-s", &serial])
                    .args(["shell", "dumpsys", "battery"]),
            )
            .await
            .ok()
            .and_then(|o| parse_battery_level(&o.stdout));

        let (data_free_mb, data_total_mb) = self
            .process
            .run(
                Command::new(adb.display().to_string())
                    .args(["-s", &serial])
                    .args(["shell", "df", "/data"]),
            )
            .await
            .ok()
            .map_or((None, None), |o| parse_df_data(&o.stdout));

        Ok(DeviceFacts {
            model,
            android_release,
            sdk_int,
            battery_pct,
            data_free_mb,
            data_total_mb,
        })
    }

    /// Live network state of a running emulator: interface addresses + open IPv4 sockets.
    ///
    /// Sources, all standard/stable formats parsed defensively:
    /// - interfaces: `ip -o addr` (one address per line: `<idx>: <iface> <family> <addr>/<prefix> …`)
    /// - sockets: `/proc/net/tcp` and `/proc/net/udp` — the kernel format documented in
    ///   `Documentation/networking/proc_net_tcp.rst` (little-endian hex `IP:PORT`, hex state, uid)
    /// - uid → package: `pm list packages -U` (`package:<name> uid:<n>`)
    ///
    /// IPv6 sockets (`/proc/net/tcp6`) are deliberately not parsed yet — the emulator's traffic
    /// is almost always IPv4 and the v6 hex encoding is a separate can of worms.
    pub async fn device_network(&self, id: &EmulatorId) -> Result<DeviceNetwork> {
        let serial = self.running_serial(id).await?;
        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);

        let sh = |args: &[&'static str]| {
            let adb = adb.display().to_string();
            let serial = serial.clone();
            let args: Vec<&'static str> = args.to_vec();
            async move {
                self.process
                    .run(
                        Command::new(adb)
                            .args(["-s", &serial])
                            .arg("shell")
                            .args(args),
                    )
                    .await
                    .map(|o| o.stdout)
                    .unwrap_or_default()
            }
        };

        let interfaces = parse_ip_addr(&sh(&["ip", "-o", "addr"]).await);
        let uid_pkg = parse_pm_list_packages_u(&sh(&["pm", "list", "packages", "-U"]).await);
        let mut connections = parse_proc_net(&sh(&["cat", "/proc/net/tcp"]).await, "tcp", &uid_pkg);
        connections.extend(parse_proc_net(
            &sh(&["cat", "/proc/net/udp"]).await,
            "udp",
            &uid_pkg,
        ));

        Ok(DeviceNetwork {
            interfaces,
            connections,
        })
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

/// The `level: NN` line of `adb shell dumpsys battery` → percent. `dumpsys battery` prints a
/// block of `  key: value` lines; `level` is the charge percent (0-100). Anything unparseable →
/// `None`.
fn parse_battery_level(dumpsys: &str) -> Option<u8> {
    dumpsys.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("level:")?;
        rest.trim().parse().ok()
    })
}

/// `adb shell df /data` → (free MB, total MB). `df` prints a header row then one data row whose
/// columns are `Filesystem 1K-blocks Used Available Use% Mounted-on` (POSIX). Values are in
/// 1 KiB blocks. A row that doesn't have enough numeric columns → `(None, None)`.
fn parse_df_data(df: &str) -> (Option<u64>, Option<u64>) {
    let Some(row) = df.lines().nth(1) else {
        return (None, None);
    };
    let cols: Vec<&str> = row.split_whitespace().collect();
    // total = 1K-blocks (col 1), free = Available (col 3).
    let kib_to_mb = |kib: u64| kib / 1024;
    let total = cols
        .get(1)
        .and_then(|c| c.parse::<u64>().ok())
        .map(kib_to_mb);
    let free = cols
        .get(3)
        .and_then(|c| c.parse::<u64>().ok())
        .map(kib_to_mb);
    (free, total)
}

/// `ip -o addr` → one [`NetInterface`] per `inet`/`inet6` address line. With `-o`, each address
/// is on its own line: `2: wlan0    inet 10.0.2.16/24 brd … scope global wlan0 \ …`.
fn parse_ip_addr(out: &str) -> Vec<NetInterface> {
    out.lines()
        .filter_map(|line| {
            let mut toks = line.split_whitespace();
            let _idx = toks.next()?;
            let name = toks.next()?.trim_end_matches(':').to_string();
            let family = toks.next()?; // "inet" | "inet6" | "link/ether" | …
            if family != "inet" && family != "inet6" {
                return None;
            }
            let addr = toks.next()?.to_string(); // "10.0.2.16/24"
            Some(NetInterface { name, addr })
        })
        .collect()
}

/// `pm list packages -U` → uid → package. Each line is `package:<name> uid:<n>`.
fn parse_pm_list_packages_u(out: &str) -> std::collections::HashMap<u32, String> {
    let mut map = std::collections::HashMap::new();
    for line in out.lines() {
        let pkg = line
            .split_whitespace()
            .find_map(|t| t.strip_prefix("package:"));
        let uid = line
            .split_whitespace()
            .find_map(|t| t.strip_prefix("uid:"))
            .and_then(|n| n.trim().parse::<u32>().ok());
        if let (Some(pkg), Some(uid)) = (pkg, uid) {
            map.entry(uid).or_insert_with(|| pkg.to_string());
        }
    }
    map
}

/// Decode a `/proc/net/*` little-endian hex `IIIIIIII:PPPP` field into `"a.b.c.d:port"`.
/// IPv6 (32-hex) fields aren't handled — `None`.
fn decode_hex_addr(field: &str) -> Option<String> {
    let (ip_hex, port_hex) = field.split_once(':')?;
    if ip_hex.len() != 8 {
        return None; // IPv6 or malformed
    }
    let n = u32::from_str_radix(ip_hex, 16).ok()?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    Some(format!(
        "{}.{}.{}.{}:{port}",
        n & 0xff,
        (n >> 8) & 0xff,
        (n >> 16) & 0xff,
        (n >> 24) & 0xff
    ))
}

/// The hex TCP state in `/proc/net/tcp` column 3 → its name. (`Documentation/networking/proc_net_tcp.rst`)
fn tcp_state(hex: &str) -> &'static str {
    match hex {
        "01" => "ESTABLISHED",
        "02" => "SYN_SENT",
        "03" => "SYN_RECV",
        "04" => "FIN_WAIT1",
        "05" => "FIN_WAIT2",
        "06" => "TIME_WAIT",
        "07" => "CLOSE",
        "08" => "CLOSE_WAIT",
        "09" => "LAST_ACK",
        "0A" => "LISTEN",
        "0B" => "CLOSING",
        _ => "?",
    }
}

/// Parse `/proc/net/tcp` (or `udp`) into [`NetConnection`]s (IPv4 rows only). Columns after the
/// header: `sl local_address rem_address st … … … uid …`. Rows that don't parse are skipped.
fn parse_proc_net(
    out: &str,
    proto: &'static str,
    uid_pkg: &std::collections::HashMap<u32, String>,
) -> Vec<NetConnection> {
    out.lines()
        .skip(1) // "  sl  local_address rem_address   st …" header
        .filter_map(|line| {
            // cols: [ "0:", local, remote, st, tx:rx, tr:when, retrnsmt, uid, … ]
            let cols: Vec<&str> = line.split_whitespace().collect();
            let local = decode_hex_addr(cols.get(1)?)?;
            let remote = decode_hex_addr(cols.get(2)?)?;
            let state = if proto == "tcp" {
                tcp_state(cols.get(3)?)
            } else {
                ""
            };
            let uid = cols.get(7)?.parse::<u32>().ok()?;
            Some(NetConnection {
                proto,
                local,
                remote,
                state,
                uid,
                package: uid_pkg.get(&uid).cloned(),
            })
        })
        .collect()
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

        // De-duplicate against what already exists so "create Pixel 6" twice yields "Pixel 6" and
        // "Pixel 6 (2)" (on disk: `pixel_6` and `pixel_6_2`) instead of a hard "already exists"
        // error. Ground truth for AVD names is `avdmanager list avd` ∪ our registry; display
        // names come from the registry.
        let rows = self.registry.list_rows().await?;
        let mut taken_avd = self.list_avd_names().await;
        taken_avd.extend(rows.iter().map(|r| r.avd_name.clone()));
        let taken_display: Vec<String> = rows.iter().map(|r| r.display_name.clone()).collect();

        let avd_name = emu_core::profile::unique_avd_name(&spec.avd_name, &taken_avd);
        let display_name =
            emu_core::profile::unique_display_name(&spec.display_name, &taken_display);

        let cmd = Command {
            stdin: Some(Self::prompt_answer_stdin()),
            ..Command::new(avdmanager.display().to_string())
                .arg("create")
                .arg("avd")
                .args(["-n", &avd_name])
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
            avd_name,
            display_name,
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

        // Fail fast on a missing prerequisite rather than spawning the emulator and then timing
        // out for 5 minutes because `adb` (used to detect boot) doesn't exist.
        let state = self.installed_state().await?;
        if !state.is_installed(ComponentId::Emulator) {
            return Err(CoreError::Unsupported(
                "the emulator package isn't installed — install it on the Dependencies screen."
                    .to_string(),
            ));
        }
        if !state.is_installed(ComponentId::PlatformTools) {
            return Err(CoreError::Unsupported(
                "platform-tools (adb) isn't installed — Emulator Studio needs it to launch and \
                 track emulators. Install it on the Dependencies screen, then try again."
                    .to_string(),
            ));
        }

        // Desktop keyboard: `avdmanager` omits `hw.keyboard` from `config.ini`, so the emulator
        // drops host keystrokes in text fields until it reads `yes`. Fixed every launch — covers
        // AVDs we created and ones adopted from elsewhere. Best-effort.
        let kbd_note = self.ensure_hw_keyboard_for(&avd_name).await;

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
        // Device frame/bezel. `-skin <name>` + `-skindir <dir>` are the documented flags
        // (<https://developer.android.com/studio/run/emulator-commandline>). Only passed when the
        // skin is actually installed under `<sdk>/skins/<name>` — a cmdline-tools-only SDK has no
        // `skins/` dir, and passing the flags anyway makes the emulator print a scary warning and
        // still draw no frame. When it's missing we say so on the job log instead.
        let mut skin_note: Option<String> = None;
        if let Some(skin) = &opts.skin {
            let skins_dir = sdk_root.join("skins");
            if self.fs.exists(&skins_dir.join(skin)).await? {
                cmd = cmd
                    .args(["-skin", skin])
                    .args(["-skindir", &skins_dir.display().to_string()]);
            } else {
                skin_note = Some(format!(
                    "device-frame skin '{skin}' isn't installed under {} — launching without a \
                     frame",
                    skins_dir.display()
                ));
            }
        }
        cmd = cmd.args(opts.extra_args.iter().cloned());

        // Fresh log file for this run: `<data_dir>/logs/<avd>.log`.
        let log_path = self.log_path(&avd_name);
        let _ = self.fs.ensure_dir(&self.data_dir.join("logs")).await;
        let _ = self.fs.write_atomic(&log_path, b"").await;

        if let Some(note) = kbd_note {
            self.emit_log(job, &log_path, note).await;
        }
        if let Some(note) = skin_note {
            self.emit_log(job, &log_path, note).await;
        }
        self.emit_log(job, &log_path, format!("launching {}", cmd.display()))
            .await;
        let child = self.process.spawn(cmd).await?;
        let pid = child.pid();
        let child: ChildHandle = Arc::new(AsyncMutex::new(child));

        let serial = match self
            .wait_for_boot(&adb, &avd_name, &child, &log_path, job)
            .await
        {
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

        self.emit_log(job, &log_path, format!("{avd_name} booted ({serial})"))
            .await;
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

    /// Delete a tracked emulator. `wipe` also removes the AVD from disk
    /// (`avdmanager delete avd -n <name>` — which removes the whole `.avd` dir, userdata included);
    /// `wipe = false` just untracks it (the registry row goes, the AVD stays and `reconcile()`
    /// would re-adopt it as `discovered`). Refuses while the emulator is running.
    async fn delete(&self, id: EmulatorId, wipe: bool) -> Result<()> {
        let row = self
            .registry
            .get_row(id.as_str())
            .await?
            .ok_or_else(|| CoreError::NotFound {
                what: "emulator",
                name: id.to_string(),
            })?;

        let sdk_root = self.sdk_root().await?;
        let adb = self.adb_path(&sdk_root);
        if self.serial_for_avd(&adb, &row.avd_name).await?.is_some() {
            return Err(CoreError::invalid(
                "emulator",
                format!(
                    "'{}' is running — stop it before deleting",
                    row.display_name
                ),
            ));
        }

        if wipe {
            let avdmanager = self.avdmanager_path(&sdk_root);
            let cmd = Command::new(avdmanager.display().to_string())
                .args(["delete", "avd"])
                .args(["-n", &row.avd_name]);
            let out = self.process.run(cmd).await?;
            // "There is no Android Virtual Device named '<name>'." — already gone out-of-band; the
            // end state (no AVD) is what we wanted, so treat it as success.
            if !out.success() && !out.stderr.contains("no Android Virtual Device named") {
                return Err(CoreError::Process {
                    program: "avdmanager delete avd".to_string(),
                    code: out.status,
                    stderr: out.stderr,
                });
            }
        }

        self.registry.delete_row(id.as_str()).await?;
        Ok(())
    }

    /// Make the registry match on-disk / running reality and return the refreshed live state of
    /// every tracked emulator.
    ///
    /// - **Adopt**: a loadable AVD from `avdmanager list avd` with no registry row is inserted as
    ///   [`EmulatorSource::Manual`]`{ discovered: true }`, enriched from its `config.ini`.
    /// - **Flag, don't drop**: a registry row whose AVD is missing from `avdmanager list avd`, or
    ///   present but un-loadable (missing system image), is set to [`RunState::Error`]. Hard
    ///   deletion stays explicit — that is [`Provider::delete`].
    /// - **Refresh**: every surviving row's `last_state` / `adb_serial` / `grpc_port` / `pid` are
    ///   re-derived from `adb`.
    /// - **Kill-safety**: a row left `Booting`/`Running` with a `pid` from a previous app run whose
    ///   emulator is no longer in `adb devices` is reset to `Stopped` and its `pid` cleared. (A
    ///   [`ProcessRunner`] can't test an arbitrary pid for liveness, so "not in `adb devices`" is
    ///   the liveness signal — which is also the ground truth the UI cares about.)
    async fn reconcile(&self) -> Result<Vec<LiveState>> {
        let sdk_root = self.sdk_root().await?;
        let avdmanager = self.avdmanager_path(&sdk_root);
        let adb = self.adb_path(&sdk_root);

        // Ground truth #1: on-disk AVDs.
        let list_out = self
            .process
            .run(Command::new(avdmanager.display().to_string()).args(["list", "avd"]))
            .await?;
        let avds = parse_avdmanager_list_avd(&list_out.stdout);
        let on_disk: HashMap<&str, &AvdEntry> = avds.iter().map(|a| (a.name.as_str(), a)).collect();

        // Ground truth #2: which AVDs are live, and on which serial.
        let mut serial_by_avd: HashMap<String, String> = HashMap::new();
        for serial in self.emulator_serials(&adb).await.unwrap_or_default() {
            let cmd = Command::new(adb.display().to_string())
                .args(["-s", &serial])
                .args(["emu", "avd", "name"]);
            if let Ok(out) = self.process.run(cmd).await {
                if let Some(name) = out.stdout.lines().next() {
                    serial_by_avd.insert(name.trim().to_string(), serial);
                }
            }
        }

        // Adopt on-disk AVDs the registry doesn't know yet.
        let known: HashSet<String> = self
            .registry
            .list_rows()
            .await?
            .into_iter()
            .map(|r| r.avd_name)
            .collect();
        for avd in avds
            .iter()
            .filter(|a| a.loadable && !known.contains(&a.name))
        {
            let row = self.adopt_row(avd).await;
            self.registry.upsert_emulator(&row).await?;
        }

        // Refresh every row (adopted ones included) against ground truth.
        let mut live = Vec::new();
        for row in self.registry.list_rows().await? {
            let entry = on_disk.get(row.avd_name.as_str());
            let (state, serial) = match entry {
                None => (RunState::Error, None), // AVD deleted out-of-band.
                Some(e) if !e.loadable => (RunState::Error, None), // present but broken.
                Some(_) => match serial_by_avd.get(&row.avd_name) {
                    Some(serial) if self.boot_completed(&adb, serial).await => {
                        (RunState::Running, Some(serial.clone()))
                    }
                    Some(serial) => (RunState::Booting, Some(serial.clone())),
                    None => (RunState::Stopped, None), // not live — kill-safety resets a stale pid.
                },
            };

            let running = matches!(state, RunState::Running | RunState::Booting);
            let pid = if running { row.pid } else { None };
            let grpc_port = if running { row.grpc_port } else { None };
            let launched_at = if running { row.launched_at } else { None };

            let changed = row.last_state != state
                || row.adb_serial != serial
                || (!running && (row.pid.is_some() || row.grpc_port.is_some()));
            if changed {
                self.registry
                    .set_run_fields(
                        row.id.as_str(),
                        state,
                        serial.as_deref(),
                        grpc_port,
                        pid,
                        launched_at,
                    )
                    .await?;
            }

            live.push(LiveState {
                id: row.id.clone(),
                state,
                adb_serial: serial,
                grpc_port,
                pid,
                uptime_secs: None,
            });
        }
        Ok(live)
    }
}

impl AndroidProvider {
    /// Build a registry row for an on-disk AVD `reconcile()` just discovered. Best-effort enrichment
    /// from the AVD's `config.ini` (`image.sysdir.1` → [`ImageCoord`], `hw.device.name` → device
    /// profile, `avd.ini.displayname` → display name, `hw.ramSize` → RAM); anything unreadable just
    /// falls back to a sane default, the row is still adopted.
    async fn adopt_row(&self, avd: &AvdEntry) -> EmulatorRow {
        let now = OffsetDateTime::now_utc();
        let mut row = EmulatorRow {
            id: EmulatorId::generate(),
            avd_name: avd.name.clone(),
            display_name: avd.name.clone(),
            device_profile_id: String::new(),
            image_coord: None,
            hardware: emu_core::model::emulator::Hardware::default(),
            source: EmulatorSource::Manual { discovered: true },
            tags: Vec::new(),
            notes: String::new(),
            last_state: RunState::Stopped,
            adb_serial: None,
            grpc_port: None,
            pid: None,
            created_at: now,
            updated_at: now,
            launched_at: None,
        };

        if let Some(dir) = &avd.path {
            if let Ok(bytes) = self.fs.read(&dir.join("config.ini")).await {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    let kv = parse_ini(text);
                    if let Some(v) = kv.get("avd.ini.displayname") {
                        row.display_name = (*v).to_string();
                    }
                    if let Some(v) = kv.get("hw.device.name") {
                        row.device_profile_id = (*v).to_string();
                    }
                    if let Some(v) = kv.get("image.sysdir.1") {
                        // `system-images/android-24/default/x86_64/` → `system-images;android-24;default;x86_64`
                        row.image_coord = v.trim_end_matches('/').replace('/', ";").parse().ok();
                    }
                    if let Some(mb) = kv.get("hw.ramSize").and_then(|v| v.trim().parse().ok()) {
                        row.hardware.ram_mb = mb;
                    }
                }
            }
        }
        row
    }
}

/// Parse an AVD `config.ini` / `.ini` file into its `key=value` pairs. Blank lines, `#` comments
/// and `[section]` headers are skipped; keys and values are trimmed.
fn parse_ini(text: &str) -> HashMap<&str, &str> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim(), v.trim()))
        })
        .collect()
}

/// Given an AVD `config.ini`'s current text, return the text it should be rewritten to so the
/// desktop keyboard reaches Android text fields (`hw.keyboard=yes`), or `None` if it already
/// says `yes` (nothing to do). Any existing `hw.keyboard=...` line is replaced; otherwise the
/// key is appended. Every other line — comments, blanks, order — is preserved. See
/// [`AndroidProvider::ensure_hw_keyboard`] for why `avdmanager` leaves this wrong.
fn hw_keyboard_fix(config_ini: &str) -> Option<String> {
    let already_yes = parse_ini(config_ini)
        .get("hw.keyboard")
        .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    if already_yes {
        return None;
    }
    let mut out = String::with_capacity(config_ini.len() + 16);
    for line in config_ini.lines().filter(|line| {
        line.split_once('=')
            .is_none_or(|(k, _)| k.trim() != "hw.keyboard")
    }) {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("hw.keyboard=yes\n");
    Some(out)
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
        log_path: &Path,
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
                Ok(Ok(Some(line))) => {
                    drop(guard);
                    self.emit_log(job, log_path, line).await;
                }
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
        // A complete app-managed SDK: cmdline-tools (so `sdk_root()` resolves here), platform-tools
        // and emulator (so `launch`'s prerequisite check passes). Marker files only — the fake
        // ProcessRunner supplies behaviour.
        for marker in [
            "/data/sdk/cmdline-tools/latest/bin/sdkmanager",
            "/data/sdk/platform-tools/adb",
            "/data/sdk/emulator/emulator",
        ] {
            fs.write_atomic(Path::new(marker), b"#!/bin/sh")
                .await
                .unwrap();
        }
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
    async fn create_de_duplicates_a_name_that_is_already_taken() {
        let fs = InMemoryFs::new();
        // First create runs against the original name; the second gets the `_2` suffix.
        let process = FakeProcessRunner::new()
            .on_run("avdmanager create avd -n pixel6_api34 -k", ok(""))
            .on_run("avdmanager create avd -n pixel6_api34_2 -k", ok(""));
        let (provider, dir) = provider_with(process, fs).await;

        provider.create(spec()).await.expect("first create");
        let id2 = provider.create(spec()).await.expect("second create");

        let reopened = Registry::open(dir.path()).await.expect("reopen");
        let row = reopened.get_row(id2.as_str()).await.unwrap().unwrap();
        assert_eq!(row.avd_name, "pixel6_api34_2");
        assert_eq!(row.display_name, "Pixel 6 · API 34 (2)");
    }

    #[tokio::test]
    async fn rename_de_duplicates_against_other_emulators() {
        let fs = InMemoryFs::new();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;
        let a = seed_emulator(&provider).await; // display "Pixel 6 · API 34"
                                                // A second row with a different name.
        let b = EmulatorId::generate();
        provider
            .registry
            .upsert_emulator(&EmulatorRow::new(
                b.clone(),
                "other_avd".into(),
                "Other".into(),
                "pixel_6".into(),
                coord(),
                emu_core::model::emulator::Hardware::default(),
                EmulatorSource::Manual { discovered: false },
                OffsetDateTime::now_utc(),
            ))
            .await
            .unwrap();

        // Renaming B onto A's name gets a suffix; renaming B to its own current name is a no-op.
        provider
            .rename(&b, "Pixel 6 · API 34")
            .await
            .expect("rename");
        assert_eq!(
            provider.detail(&b).await.unwrap().display_name,
            "Pixel 6 · API 34 (2)"
        );
        // A keeps its name.
        assert_eq!(
            provider.detail(&a).await.unwrap().display_name,
            "Pixel 6 · API 34"
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

    #[test]
    fn gpu_mode_values_match_the_docs() {
        assert_eq!(gpu_mode(Graphics::Auto), "auto");
        assert_eq!(gpu_mode(Graphics::Host), "host");
        assert_eq!(
            gpu_mode(Graphics::SwiftshaderIndirect),
            "swiftshader_indirect"
        );
    }

    #[test]
    fn parse_battery_level_reads_the_dumpsys_line() {
        let dumpsys = "Current Battery Service state:\n  AC powered: false\n  level: 87\n  scale: 100\n  temperature: 250\n";
        assert_eq!(parse_battery_level(dumpsys), Some(87));
        assert_eq!(parse_battery_level("no battery block here"), None);
    }

    #[test]
    fn parse_df_data_reads_available_and_total_in_mb() {
        // `df /data` on a real emulator, 1 KiB blocks.
        let df = "Filesystem     1K-blocks   Used Available Use% Mounted on\n/dev/block/dm-5  6154240 812340   5341900  14% /data\n";
        let (free, total) = parse_df_data(df);
        assert_eq!(free, Some(5_341_900 / 1024));
        assert_eq!(total, Some(6_154_240 / 1024));
        assert_eq!(parse_df_data("header only\n"), (None, None));
    }

    #[test]
    fn parse_ip_addr_keeps_only_inet_lines() {
        let out = "1: lo    inet 127.0.0.1/8 scope host lo\\       valid_lft forever\n\
                   2: wlan0    inet 10.0.2.16/24 brd 10.0.2.255 scope global wlan0\\    valid_lft\n\
                   2: wlan0    link/ether 02:00:00:00:00:00 brd ff:ff:ff:ff:ff:ff\n";
        let ifs = parse_ip_addr(out);
        assert_eq!(ifs.len(), 2);
        assert_eq!(ifs[1].name, "wlan0");
        assert_eq!(ifs[1].addr, "10.0.2.16/24");
    }

    #[test]
    fn decode_hex_addr_reverses_the_le_ipv4_and_reads_the_port() {
        // 0100007F:1F90 → 127.0.0.1:8080 ; IPv6 (32-hex) → None.
        assert_eq!(
            decode_hex_addr("0100007F:1F90").as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            decode_hex_addr("00000000:0000").as_deref(),
            Some("0.0.0.0:0")
        );
        assert_eq!(
            decode_hex_addr("00000000000000000000000001000000:0035"),
            None
        );
    }

    #[test]
    fn parse_proc_net_reads_tcp_rows_and_maps_the_uid() {
        let uid_pkg =
            parse_pm_list_packages_u("package:com.example.app uid:10123\npackage:x uid:10124\n");
        let tcp = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   \
                   0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 111 1 0 0\n   \
                   1: 1002000A:D9F2 08080808:01BB 01 00000000:00000000 00:00000000 00000000 10123        0 222 1 0 0\n";
        let conns = parse_proc_net(tcp, "tcp", &uid_pkg);
        assert_eq!(conns.len(), 2);
        assert_eq!(conns[0].state, "LISTEN");
        assert_eq!(conns[0].local, "127.0.0.1:8080");
        assert_eq!(conns[1].state, "ESTABLISHED");
        assert_eq!(conns[1].remote, "8.8.8.8:443");
        assert_eq!(conns[1].uid, 10123);
        assert_eq!(conns[1].package.as_deref(), Some("com.example.app"));
    }

    #[test]
    fn hw_keyboard_fix_appends_the_key_when_it_is_missing() {
        let out = hw_keyboard_fix("hw.ramSize=2048\nhw.gpu.enabled=yes\n")
            .expect("a config without hw.keyboard needs the fix");
        assert!(out.contains("hw.ramSize=2048"));
        assert!(out.contains("hw.gpu.enabled=yes"));
        assert!(out.ends_with("hw.keyboard=yes\n"));
    }

    #[test]
    fn hw_keyboard_fix_replaces_an_explicit_no() {
        let out = hw_keyboard_fix("hw.keyboard=no\nhw.ramSize=2048\n")
            .expect("hw.keyboard=no needs the fix");
        assert!(!out.contains("hw.keyboard=no"));
        assert_eq!(out.matches("hw.keyboard").count(), 1);
        assert!(out.contains("hw.keyboard=yes"));
        assert!(out.contains("hw.ramSize=2048"));
    }

    #[test]
    fn hw_keyboard_fix_is_a_no_op_when_already_yes() {
        assert_eq!(hw_keyboard_fix("hw.keyboard=yes\nhw.ramSize=2048\n"), None);
        assert_eq!(hw_keyboard_fix("hw.keyboard = YES\n"), None);
    }

    #[tokio::test]
    async fn launch_enables_the_hardware_keyboard_in_config_ini() {
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/data/avd/pixel6_api34.avd/config.ini"),
            b"hw.device.name=pixel_6\nhw.keyboard=no\n",
        )
        .await
        .unwrap();
        let process = FakeProcessRunner::new()
            .on_run(
                "avdmanager list avd",
                ok(
                    "Available Android Virtual Devices:\n    Name: pixel6_api34\n  Device: \
                    pixel_6 (Google)\n    Path: /data/avd/pixel6_api34.avd\n",
                ),
            )
            .on_spawn("emulator @pixel6_api34", ["emulator: boot started"], ok(""))
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        provider
            .launch(id, LaunchOpts::default(), &job())
            .await
            .expect("launch");

        let config = provider
            .fs
            .read(Path::new("/data/avd/pixel6_api34.avd/config.ini"))
            .await
            .expect("config.ini still there");
        let text = String::from_utf8(config).unwrap();
        assert!(text.contains("hw.keyboard=yes"), "got: {text}");
        assert!(!text.contains("hw.keyboard=no"));
        assert!(text.contains("hw.device.name=pixel_6"));
    }

    #[tokio::test]
    async fn logcat_start_streams_lines_until_stopped() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("emu avd name", ok("pixel6_api34\nOK\n"))
            .on_spawn_lingering(
                "logcat -v threadtime",
                [
                    "01-02 03:04:05.678  1234  1234 I ActivityManager: Start proc",
                    "01-02 03:04:06.001  1234  1300 W Choreographer: skipped frames",
                ],
                ok(""),
            );
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        let handle = provider.logcat_start(&id).await.expect("logcat starts");
        let mut got = Vec::new();
        {
            let mut child = handle.lock().await;
            while let Ok(Some(line)) = child.next_line().await {
                got.push(line);
                if got.len() == 2 {
                    break;
                }
            }
        }
        assert!(got[0].contains("ActivityManager"));
        assert!(got[1].contains("Choreographer"));

        provider.logcat_stop(&id).await;
        // A second stop is a no-op, not a panic.
        provider.logcat_stop(&id).await;
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
    async fn launch_fails_clearly_when_platform_tools_is_missing() {
        let fs = InMemoryFs::new();
        // cmdline-tools + emulator, but NO platform-tools/adb.
        fs.write_atomic(
            Path::new("/data/sdk/cmdline-tools/latest/bin/sdkmanager"),
            b"#!/bin/sh",
        )
        .await
        .unwrap();
        fs.write_atomic(Path::new("/data/sdk/emulator/emulator"), b"#!/bin/sh")
            .await
            .unwrap();
        // `provider_with` would add adb — build the provider directly instead.
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::open(dir.path()).await.unwrap();
        let provider = AndroidProvider::new(
            Arc::new(FakeProcessRunner::new()),
            Arc::new(fs),
            registry,
            PathBuf::from("/data"),
            HostOs::Linux,
        );
        let id = seed_emulator(&provider).await;

        let err = provider
            .launch(id, LaunchOpts::default(), &job())
            .await
            .expect_err("no adb → launch must refuse");
        match err {
            CoreError::Unsupported(msg) => assert!(msg.contains("platform-tools")),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn launch_adds_skin_flags_when_the_frame_skin_is_installed() {
        let fs = InMemoryFs::new();
        fs.write_atomic(Path::new("/data/sdk/skins/pixel_6/layout"), b"skin")
            .await
            .unwrap();
        let process = FakeProcessRunner::new()
            .on_spawn(
                "emulator @pixel6_api34 -skin pixel_6 -skindir /data/sdk/skins",
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
            skin: Some("pixel_6".into()),
            ..LaunchOpts::default()
        };
        provider.launch(id, opts, &job()).await.expect("launch");
        // `on_spawn` needle matches only if `-skin` + `-skindir` were in the argv.
    }

    #[tokio::test]
    async fn launch_skips_the_skin_when_it_is_not_installed_and_says_so() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_spawn("emulator @pixel6_api34", Vec::<String>::new(), ok(""))
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("shell getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        let (jh, log) = JobHandle::collector(JobId("j".into()));
        let opts = LaunchOpts {
            skin: Some("pixel_6".into()),
            ..LaunchOpts::default()
        };
        provider.launch(id, opts, &jh).await.expect("launch");

        let said_missing = log
            .lock()
            .unwrap()
            .iter()
            .filter_map(|p| p.log_line.clone())
            .any(|l| l.contains("isn't installed"));
        assert!(
            said_missing,
            "a missing frame skin is called out on the log"
        );
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

    // ---- per-emulator log tee / tail (task 0021) -------------------------------------------

    #[tokio::test]
    async fn launch_tees_output_to_a_log_file_that_read_log_tail_returns() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_spawn(
                "emulator @pixel6_api34",
                ["emulator: line one", "emulator: line two"],
                ok(""),
            )
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            // "0" on the first poll so the loop drains the child's lines before the EOF-triggered
            // final boot check consumes the "1".
            .on_run("shell getprop sys.boot_completed", ok("0\n"))
            .on_run("shell getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_emulator(&provider).await;

        provider
            .launch(id.clone(), LaunchOpts::default(), &job())
            .await
            .expect("launch");

        let tail = provider.read_log_tail(&id, 100).await.expect("tail");
        assert!(tail.first().unwrap().contains("launching"));
        assert!(tail.iter().any(|l| l.contains("line one")));
        assert!(tail.iter().any(|l| l.contains("line two")));
        assert!(tail.iter().any(|l| l.contains("booted")));

        // `max_lines` caps from the end.
        let last_two = provider.read_log_tail(&id, 2).await.expect("tail 2");
        assert_eq!(last_two.len(), 2);
        assert!(last_two.iter().any(|l| l.contains("booted")));
    }

    #[tokio::test]
    async fn read_log_tail_is_empty_for_a_never_launched_emulator() {
        let fs = InMemoryFs::new();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;
        let id = seed_emulator(&provider).await;
        assert!(provider
            .read_log_tail(&id, 50)
            .await
            .expect("tail")
            .is_empty());
    }

    #[tokio::test]
    async fn read_log_tail_of_an_unknown_id_is_not_found() {
        let fs = InMemoryFs::new();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;
        let err = provider
            .read_log_tail(&EmulatorId::generate(), 10)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    // ---- reconcile / delete (task 0019) --------------------------------------------------

    /// Seed a full registry row with a chosen name / last-state / pid.
    async fn seed_row(
        provider: &AndroidProvider,
        name: &str,
        state: RunState,
        pid: Option<u32>,
    ) -> EmulatorId {
        let id = EmulatorId::generate();
        let now = OffsetDateTime::now_utc();
        let row = EmulatorRow {
            id: id.clone(),
            avd_name: name.to_string(),
            display_name: name.to_string(),
            device_profile_id: "pixel_6".into(),
            image_coord: Some(coord()),
            hardware: emu_core::model::emulator::Hardware::default(),
            source: EmulatorSource::Manual { discovered: false },
            tags: Vec::new(),
            notes: String::new(),
            last_state: state,
            adb_serial: pid.map(|_| "emulator-5554".to_string()),
            grpc_port: None,
            pid,
            created_at: now,
            updated_at: now,
            launched_at: None,
        };
        provider
            .registry
            .upsert_emulator(&row)
            .await
            .expect("seed row");
        id
    }

    /// Synthesize `avdmanager list avd` output for the given loadable + broken AVD names.
    fn list_avd(loadable: &[&str], broken: &[&str]) -> String {
        use std::fmt::Write as _;
        let mut s = String::from("Available Android Virtual Devices:\n");
        for (i, name) in loadable.iter().enumerate() {
            if i > 0 {
                s.push_str("---------\n");
            }
            let _ = write!(
                s,
                "    Name: {name}\n  Device: pixel_6 (Google)\n    Path: /data/avd/{name}.avd\n  \
                 Target:\n          Based on: Android 7.0 (\"Nougat\") Tag/ABI: default/x86_64\n  \
                 Sdcard: 512 MB\n"
            );
        }
        if !broken.is_empty() {
            s.push_str("\nThe following Android Virtual Devices could not be loaded:\n");
            for name in broken {
                let _ = write!(
                    s,
                    "    Name: {name}\n    Path: /data/avd/{name}.avd\n   Error: Missing system \
                     image android-99/default/x86_64.\n"
                );
            }
        }
        s
    }

    /// Synthesize `adb devices` output; serial `emulator-(5554 + 2i)` for `running[i]`.
    fn devices(running: &[&str]) -> String {
        use std::fmt::Write as _;
        let mut s = String::from("List of devices attached\n");
        for i in 0..running.len() {
            let _ = writeln!(s, "emulator-{}\tdevice", 5554 + i * 2);
        }
        s
    }

    #[tokio::test]
    async fn reconcile_adopts_unknown_avds_and_flags_missing_or_broken_ones() {
        let fs = InMemoryFs::new();
        // config.ini for the AVD we expect to adopt, so enrichment is exercised too.
        fs.write_atomic(
            Path::new("/data/avd/adopt_me.avd/config.ini"),
            b"avd.ini.displayname=Adopted Pixel\nhw.device.name=pixel_6\n\
              image.sysdir.1=system-images/android-30/google_apis/x86_64/\nhw.ramSize=3072\n",
        )
        .await
        .unwrap();
        let process = FakeProcessRunner::new()
            .on_run(
                "list avd",
                ok(list_avd(&["known_ok", "adopt_me"], &["known_broken"])),
            )
            .on_run("adb devices", ok("List of devices attached\n"));
        let (provider, _dir) = provider_with(process, fs).await;

        let ok_id = seed_row(&provider, "known_ok", RunState::Stopped, None).await;
        let gone_id = seed_row(&provider, "known_gone", RunState::Running, Some(123)).await;
        let broken_id = seed_row(&provider, "known_broken", RunState::Stopped, None).await;

        let live = provider.reconcile().await.expect("reconcile");

        let state_of = |id: &EmulatorId| live.iter().find(|l| &l.id == id).unwrap().state;
        assert_eq!(state_of(&ok_id), RunState::Stopped);
        assert_eq!(
            state_of(&gone_id),
            RunState::Error,
            "AVD vanished from disk"
        );
        assert_eq!(
            state_of(&broken_id),
            RunState::Error,
            "present but un-loadable"
        );

        // The unknown loadable AVD was adopted, enriched from its config.ini.
        let rows = provider.registry.list_rows().await.unwrap();
        let adopted = rows
            .iter()
            .find(|r| r.avd_name == "adopt_me")
            .expect("adopted");
        assert_eq!(adopted.display_name, "Adopted Pixel");
        assert_eq!(adopted.device_profile_id, "pixel_6");
        assert_eq!(adopted.hardware.ram_mb, 3072);
        assert_eq!(
            adopted.image_coord.map(|c| c.to_string()).as_deref(),
            Some("system-images;android-30;google_apis;x86_64")
        );
        assert!(matches!(
            adopted.source,
            EmulatorSource::Manual { discovered: true }
        ));
        // The broken AVD is NOT adopted as a new row.
        assert_eq!(
            rows.iter().filter(|r| r.avd_name == "known_broken").count(),
            1
        );
    }

    #[tokio::test]
    async fn reconcile_kill_safety_resets_a_stale_running_row() {
        let fs = InMemoryFs::new();
        // Row thinks it's Running with a pid from a previous app run; adb shows it is NOT up.
        let process = FakeProcessRunner::new()
            .on_run("list avd", ok(list_avd(&["crashed", "other"], &[])))
            .on_run("adb devices", ok(devices(&["other"])))
            .on_run("emu avd name", ok("other\nOK\n"))
            .on_run("getprop sys.boot_completed", ok("1\n"));
        let (provider, _dir) = provider_with(process, fs).await;

        let crashed = seed_row(&provider, "crashed", RunState::Running, Some(4242)).await;
        let other = seed_row(&provider, "other", RunState::Stopped, None).await;

        provider.reconcile().await.expect("reconcile");

        let crashed_row = provider
            .registry
            .get_row(crashed.as_str())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(crashed_row.last_state, RunState::Stopped);
        assert_eq!(crashed_row.pid, None, "stale pid cleared");
        assert_eq!(crashed_row.adb_serial, None);

        let other_row = provider
            .registry
            .get_row(other.as_str())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(other_row.last_state, RunState::Running);
        assert_eq!(other_row.adb_serial.as_deref(), Some("emulator-5554"));
    }

    #[tokio::test]
    async fn reconcile_reports_booting_when_not_yet_boot_completed() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run("list avd", ok(list_avd(&["warming"], &[])))
            .on_run("adb devices", ok(devices(&["warming"])))
            .on_run("emu avd name", ok("warming\nOK\n"))
            .on_run("getprop sys.boot_completed", ok("0\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_row(&provider, "warming", RunState::Stopped, None).await;

        let live = provider.reconcile().await.expect("reconcile");
        assert_eq!(
            live.iter().find(|l| l.id == id).unwrap().state,
            RunState::Booting
        );
    }

    #[tokio::test]
    async fn delete_with_wipe_runs_avdmanager_and_drops_the_row() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run("adb devices", ok("List of devices attached\n")) // not running
            .on_run("delete avd -n gonezo", ok("\nAVD 'gonezo' deleted.\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_row(&provider, "gonezo", RunState::Stopped, None).await;

        provider.delete(id.clone(), true).await.expect("delete");
        assert!(provider
            .registry
            .get_row(id.as_str())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn delete_without_wipe_only_untracks() {
        let fs = InMemoryFs::new();
        let process =
            FakeProcessRunner::new().on_run("adb devices", ok("List of devices attached\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_row(&provider, "keepavd", RunState::Stopped, None).await;

        // Only `adb devices` is scripted. If delete(wipe=false) shelled out to
        // `avdmanager delete avd`, the fake would return an unmatched-command error and this
        // `expect` would panic — so a clean success proves the untrack path skips it.
        provider.delete(id.clone(), false).await.expect("untrack");
        assert!(provider
            .registry
            .get_row(id.as_str())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn delete_is_refused_while_running() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run(
                "adb devices",
                ok("List of devices attached\nemulator-5554\tdevice\n"),
            )
            .on_run("emu avd name", ok("busy\nOK\n"));
        let (provider, _dir) = provider_with(process, fs).await;
        let id = seed_row(&provider, "busy", RunState::Running, Some(9)).await;

        let err = provider.delete(id, true).await.unwrap_err();
        assert_eq!(err.code(), "invalid");
        assert!(err.to_string().contains("stop it before deleting"));
    }

    #[tokio::test]
    async fn delete_of_an_unknown_id_is_not_found() {
        let fs = InMemoryFs::new();
        let (provider, _dir) = provider_with(FakeProcessRunner::new(), fs).await;
        let err = provider
            .delete(EmulatorId::generate(), true)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    /// The M3 definition-of-done check: over many pseudo-random arrangements of on-disk / broken /
    /// running AVDs, the registry's `last_state` for every row converges to ground truth after a
    /// single `reconcile()`, and every loadable on-disk AVD ends up tracked.
    #[tokio::test]
    async fn reconcile_converges_to_ground_truth_over_random_scenarios() {
        const POOL: [&str; 6] = ["avd0", "avd1", "avd2", "avd3", "avd4", "avd5"];
        // xorshift64 — deterministic, no dependency.
        let mut rng: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };

        for iter in 0..48 {
            let mut loadable: Vec<&str> = Vec::new();
            let mut broken: Vec<&str> = Vec::new();
            let mut running: Vec<&str> = Vec::new();
            for name in POOL {
                match next() % 3 {
                    0 => {
                        loadable.push(name);
                        if next() & 1 == 0 {
                            running.push(name);
                        }
                    }
                    1 => broken.push(name),
                    _ => {} // absent from disk
                }
            }

            let mut process = FakeProcessRunner::new()
                .on_run("list avd", ok(list_avd(&loadable, &broken)))
                .on_run("adb devices", ok(devices(&running)));
            for name in &running {
                process = process.on_run("emu avd name", ok(format!("{name}\nOK\n")));
            }
            for _ in &running {
                process = process.on_run("getprop sys.boot_completed", ok("1\n"));
            }
            let (provider, _dir) = provider_with(process, InMemoryFs::new()).await;

            // Pre-seed rows for the first four of the pool (Stopped).
            for name in &POOL[..4] {
                seed_row(&provider, name, RunState::Stopped, None).await;
            }

            provider.reconcile().await.expect("reconcile");

            let expected = |name: &str| -> RunState {
                if running.contains(&name) {
                    RunState::Running
                } else if loadable.contains(&name) {
                    RunState::Stopped
                } else {
                    RunState::Error // broken or absent
                }
            };

            let rows = provider.registry.list_rows().await.unwrap();
            for row in &rows {
                assert_eq!(
                    row.last_state,
                    expected(&row.avd_name),
                    "iter {iter}: {} — loadable={loadable:?} broken={broken:?} running={running:?}",
                    row.avd_name
                );
            }
            // Every loadable AVD is tracked (pre-seeded or adopted).
            for name in &loadable {
                assert!(
                    rows.iter().any(|r| &r.avd_name == name),
                    "iter {iter}: loadable {name} was not adopted"
                );
            }
        }
    }
}
