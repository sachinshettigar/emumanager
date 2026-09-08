//! The one shared [`AndroidProvider`] the app holds for its whole lifetime.
//!
//! Built lazily on first use and kept as [`tauri::State`], so the provider's spawned-emulator
//! child-handle map (task `0016`) persists across commands — that is what lets `stop_emulator`
//! force-kill a stuck emulator and the app reap every child on exit (`RunEvent::ExitRequested` in
//! `lib.rs`). Task `0017` built a fresh provider per command; this is M3's shared replacement.

use std::path::PathBuf;
use std::sync::Arc;

use emu_android::provider::AndroidProvider;
use emu_core::model::HostOs;
use emu_core::registry::Registry;
use tokio::sync::OnceCell;

use crate::ipc_error::IpcError;
use crate::ports::{NativeFs, NativeProcessRunner};

/// Lazily-built, process-lifetime [`AndroidProvider`]. Managed via `app.manage()` in `lib.rs`.
pub struct ManagedProvider {
    cell: OnceCell<Arc<AndroidProvider>>,
    /// `None` when the host OS isn't supported or the data dir couldn't be resolved at startup —
    /// [`Self::get`] then returns a clear error rather than the app failing to launch.
    init: Option<(PathBuf, HostOs)>,
}

impl ManagedProvider {
    /// Record the inputs a provider needs; nothing is built yet. `data_dir`/`os` are `None` when
    /// the host is unsupported or has no writable app-data directory.
    #[must_use]
    pub fn new(data_dir: Option<PathBuf>, os: Option<HostOs>) -> Self {
        Self {
            cell: OnceCell::new(),
            init: data_dir.zip(os),
        }
    }

    /// The shared provider, building (and migrating the registry) on the first call.
    ///
    /// # Errors
    /// [`IpcError`] `unsupported` when the host can't run the SDK; a registry-open error otherwise.
    pub async fn get(&self) -> Result<Arc<AndroidProvider>, IpcError> {
        let (data_dir, os) = self.init.as_ref().ok_or_else(|| {
            IpcError::new(
                "unsupported",
                "this machine can't run the Android SDK (unsupported OS or no writable data dir)",
            )
        })?;
        self.cell
            .get_or_try_init(|| async {
                let registry = Registry::open(data_dir).await.map_err(IpcError::from)?;
                Ok::<_, IpcError>(Arc::new(AndroidProvider::new(
                    Arc::new(NativeProcessRunner),
                    Arc::new(NativeFs),
                    registry,
                    data_dir.clone(),
                    *os,
                )))
            })
            .await
            .cloned()
    }

    /// The provider if it has already been built, without building it — for the shutdown reaper,
    /// which must not spin up a provider just to tear it down.
    #[must_use]
    pub fn peek(&self) -> Option<Arc<AndroidProvider>> {
        self.cell.get().cloned()
    }
}
