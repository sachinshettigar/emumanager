//! Process-wide logging setup (task 0029).
//!
//! `AGENTS.md` §7: `tracing` everywhere, never `println!`. This wires a global subscriber with a
//! daily-rotating file layer under `<data_dir>/logs/` plus — only in debug builds — a stderr
//! layer. The filter comes from `RUST_LOG`, defaulting to `info`.
//!
//! [`init`] is called once, early in [`crate::run`]'s `setup`. It returns a
//! [`tracing_appender::non_blocking::WorkerGuard`] that MUST be kept alive for the process
//! lifetime (dropping it flushes and stops the writer thread) — `setup` hands it to
//! `app.manage` so it lives as long as the app.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{fmt, EnvFilter};

/// The rotating log file's base name. `tracing-appender` appends `.YYYY-MM-DD`.
pub const LOG_FILE_STEM: &str = "emulator-studio.log";

/// Initialise the global `tracing` subscriber.
///
/// - `data_dir`: the app data directory. `None` (unsupported host / no writable dir) → file
///   logging is skipped and only the debug stderr layer is installed.
/// - Returns the writer guard to keep alive, or `None` when no file layer was installed.
///
/// Safe to call at most once per process; a second call is a no-op (the global default is
/// already set) and logs a warning.
#[must_use]
pub fn init(data_dir: Option<&Path>) -> Option<WorkerGuard> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hyper=warn"));

    // Debug builds also echo to stderr so `just dev` shows logs in the terminal.
    let stderr_layer = cfg!(debug_assertions).then(|| fmt::layer().with_writer(std::io::stderr));

    let (file_layer, guard) = match data_dir.map(|d| d.join("logs")) {
        Some(log_dir) => match std::fs::create_dir_all(&log_dir) {
            Ok(()) => {
                let appender = tracing_appender::rolling::daily(&log_dir, LOG_FILE_STEM);
                let (writer, guard) = tracing_appender::non_blocking(appender);
                let layer = fmt::layer()
                    .with_ansi(false)
                    .with_target(true)
                    .with_writer(writer);
                (Some(layer), Some(guard))
            }
            Err(e) => {
                eprintln!(
                    "emulator-studio: could not create {}: {e}",
                    log_dir.display()
                );
                (None, None)
            }
        },
        None => (None, None),
    };

    let already_set = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
        .is_err();

    if already_set {
        tracing::warn!("logging::init called twice — keeping the first subscriber");
        return None;
    }

    if guard.is_some() {
        tracing::info!(
            log_dir = %data_dir.map_or_else(
                || "<none>".to_string(),
                |d| d.join("logs").display().to_string()
            ),
            "logging initialised"
        );
    } else {
        tracing::info!("logging initialised (stderr only — no writable data dir)");
    }
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_with_no_data_dir_returns_no_guard_and_does_not_panic() {
        // Runs in the test process where a subscriber may already be set by another test — either
        // way this must not panic and must not hand back a file guard.
        let guard = init(None);
        assert!(guard.is_none());
    }
}
