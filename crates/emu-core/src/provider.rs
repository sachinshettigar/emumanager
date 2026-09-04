//! The [`Provider`] trait — the seam between orchestration and a concrete emulator backend.
//!
//! `emu-android` is the only implementation today (it drives `sdkmanager` / `avdmanager` /
//! `emulator` / `adb` through a [`ProcessRunner`](crate::ports::ProcessRunner)). The trait is
//! kept backend-neutral so a remote-Mac or cloud provider could slot in later
//! (`docs/architecture.md` deferred items).
//!
//! There is **no implementation in this crate** — task `0002` only defines the shape.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::device::DeviceProfile;
use crate::model::emulator::{EmulatorId, Graphics, LiveState};
use crate::model::image::{ImageCoord, ImageFilter, SystemImage};
use crate::model::job::JobHandle;
use crate::model::plan::CreateSpec;

/// Options for a launch that aren't baked into the AVD. `default()` = windowed, warm boot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOpts {
    /// Run without a window (CI / headless hosts).
    pub headless: bool,
    /// Wipe user data on this boot (`-wipe-data`).
    pub wipe_data: bool,
    /// Skip snapshots and boot cold (`-no-snapshot-load`).
    pub cold_boot: bool,
    /// Override the AVD's graphics mode for this launch only.
    pub graphics: Option<Graphics>,
    /// Extra raw flags passed through to the `emulator` binary (advanced/escape hatch).
    pub extra_args: Vec<String>,
}

/// A handle to a launched emulator process, returned by [`Provider::launch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningHandle {
    /// Which tracked emulator this is.
    pub id: EmulatorId,
    /// adb serial once the device appears (e.g. `emulator-5554`).
    pub adb_serial: Option<String>,
    /// The `emulator` process id.
    pub pid: Option<u32>,
    /// gRPC control port, when known.
    pub grpc_port: Option<u16>,
}

/// A concrete emulator backend.
///
/// Implementations must be cancellation-aware: every method may be dropped mid-flight and must
/// leave the system in a state `reconcile` can recover from.
#[async_trait]
pub trait Provider: Send + Sync {
    /// List available device hardware profiles (`avdmanager list device`).
    async fn list_devices(&self) -> Result<Vec<DeviceProfile>>;

    /// List system images matching `filter` (`sdkmanager --list` + repo XML).
    async fn list_images(&self, filter: ImageFilter) -> Result<Vec<SystemImage>>;

    /// Ensure the given image is installed, downloading it if needed (progress on `job`).
    async fn ensure_image(&self, coord: ImageCoord, job: &JobHandle) -> Result<()>;

    /// Create an AVD from `spec` and return the new tracked id.
    async fn create(&self, spec: CreateSpec) -> Result<EmulatorId>;

    /// Launch an emulator and wait until it is booted (progress + logs on `job`).
    async fn launch(
        &self,
        id: EmulatorId,
        opts: LaunchOpts,
        job: &JobHandle,
    ) -> Result<RunningHandle>;

    /// Stop a running emulator gracefully.
    async fn stop(&self, id: EmulatorId) -> Result<()>;

    /// Delete an AVD. When `wipe` is false the tracked row is removed but the on-disk AVD is
    /// kept (useful when the user wants to hand it to Android Studio).
    async fn delete(&self, id: EmulatorId, wipe: bool) -> Result<()>;

    /// Reconcile tracked state against reality: adopt out-of-band AVDs, drop vanished ones,
    /// refresh live state. Returns the current live state of everything known.
    async fn reconcile(&self) -> Result<Vec<LiveState>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_opts_default_is_windowed_and_warm() {
        let o = LaunchOpts::default();
        assert!(!o.headless);
        assert!(!o.wipe_data);
        assert!(!o.cold_boot);
        assert!(o.graphics.is_none());
        assert!(o.extra_args.is_empty());
    }

    #[test]
    fn provider_trait_is_object_safe() {
        // Compiles only if `Provider` is dyn-compatible.
        fn _assert(_: &dyn Provider) {}
    }

    #[test]
    fn running_handle_round_trips() {
        let h = RunningHandle {
            id: EmulatorId("e1".into()),
            adb_serial: Some("emulator-5554".into()),
            pid: Some(4321),
            grpc_port: Some(8554),
        };
        let json = serde_json::to_string(&h).expect("ser");
        let back: RunningHandle = serde_json::from_str(&json).expect("de");
        assert_eq!(h, back);
    }
}
