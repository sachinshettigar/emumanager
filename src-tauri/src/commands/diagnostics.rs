//! `export_diagnostics` (task 0029): bundle the recent app log, a fresh host report, version
//! info, and the (redacted) tracked-emulators table into a single zip the user can attach to a
//! bug report. Returns the zip path; the UI then `reveal_path`s it.

use std::io::Write as _;
use std::path::Path;

use emu_core::ports::HostProbe as _;
use serde_json::json;
use tauri::{AppHandle, Manager as _, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use zip::write::SimpleFileOptions;

use crate::ipc_error::IpcError;
use crate::provider_state::ManagedProvider;

/// Replace the current user's home-directory prefix with `~` everywhere it appears in `text`.
///
/// This is the only scrubbing applied: the app's own `tracing` output is authored not to contain
/// credentials, so the real leak to guard against is the **username** embedded in absolute paths
/// (`/Users/alice/…`, `C:\Users\alice\…`). Anything a third-party tool prints into the log that
/// happens to look secret is *not* detected — documented limitation.
fn redact(text: &str) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .filter(|h| !h.is_empty());
    redact_with(text, home.as_deref())
}

/// [`redact`] with the home directory passed in, so it's testable without touching the
/// environment.
fn redact_with(text: &str, home: Option<&str>) -> String {
    match home.filter(|h| !h.is_empty()) {
        Some(h) => text.replace(h, "~"),
        None => text.to_string(),
    }
}

/// The last `max_lines` lines of the file at `path`, or `""` if it can't be read.
fn tail(path: &Path, max_lines: usize) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

/// The newest `emulator-studio.log.*` file under `log_dir`, if any (the appender rotates daily and
/// suffixes the date, which sorts lexicographically).
fn newest_log_file(log_dir: &Path) -> Option<std::path::PathBuf> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(log_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(crate::logging::LOG_FILE_STEM))
        })
        .collect();
    files.sort();
    files.pop()
}

/// Write a redacted diagnostics zip to `<data_dir>/diagnostics-<unix-ts>.zip`.
#[tauri::command]
#[specta::specta]
pub async fn export_diagnostics(
    app: AppHandle,
    mgr: State<'_, ManagedProvider>,
) -> Result<String, IpcError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| IpcError::new("fs_error", format!("resolving the data directory: {e}")))?;

    // 1. log tail (redacted)
    let log_tail = newest_log_file(&data_dir.join("logs"))
        .map(|p| redact(&tail(&p, 2000)))
        .unwrap_or_default();

    // 2. host report — a fresh probe, best-effort
    let host_json = match emu_host::NativeHostProbe::new(data_dir.clone())
        .inspect()
        .await
    {
        Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_else(|_| "{}".to_string()),
        Err(e) => json!({ "error": e.to_string() }).to_string(),
    };

    // 3. versions
    let versions = json!({
        "app": env!("CARGO_PKG_VERSION"),
        "tauri": tauri::VERSION,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    // 4. tracked emulators — hand-built so only these fields leave the machine (no notes, no tags)
    let mut emulators = Vec::new();
    if let Ok(provider) = mgr.get().await {
        if let Ok(rows) = provider.registry().list_rows().await {
            for r in rows {
                emulators.push(json!({
                    "id": r.id.as_str(),
                    "avdName": r.avd_name,
                    "displayName": r.display_name,
                    "deviceProfileId": r.device_profile_id,
                    "imageCoord": r.image_coord.map(|c| c.to_string()),
                    "lastState": format!("{:?}", r.last_state),
                    "adbSerial": r.adb_serial,
                    "createdAt": r.created_at.format(&Rfc3339).unwrap_or_default(),
                    "updatedAt": r.updated_at.format(&Rfc3339).unwrap_or_default(),
                }));
            }
        }
    }

    // 5. write the zip
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let zip_path = data_dir.join(format!("diagnostics-{ts}.zip"));
    let file = std::fs::File::create(&zip_path)
        .map_err(|e| IpcError::new("fs_error", format!("creating {}: {e}", zip_path.display())))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let entries: [(&str, String); 4] = [
        ("emulator-studio.log", log_tail),
        ("host-report.json", host_json),
        (
            "versions.json",
            serde_json::to_string_pretty(&versions).unwrap_or_default(),
        ),
        (
            "emulators.json",
            serde_json::to_string_pretty(&emulators).unwrap_or_default(),
        ),
    ];
    for (name, body) in &entries {
        let zip_err = |e: &dyn std::fmt::Display| {
            IpcError::new("fs_error", format!("writing {name} into the zip: {e}"))
        };
        zip.start_file(*name, opts).map_err(|e| zip_err(&e))?;
        zip.write_all(body.as_bytes()).map_err(|e| zip_err(&e))?;
    }
    zip.finish()
        .map_err(|e| IpcError::new("fs_error", format!("finishing the zip: {e}")))?;

    tracing::info!(path = %zip_path.display(), "wrote diagnostics bundle");
    Ok(zip_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_collapses_the_home_path_prefix() {
        let line = "2026-09-08 opened /Users/testuser/Library/Application Support/x/logs/a.log \
                    token=abc123";
        let out = redact_with(line, Some("/Users/testuser"));
        assert!(out.contains("~/Library/Application Support/x/logs/a.log"));
        assert!(!out.contains("/Users/testuser"));
        // Documented limitation: non-path secrets are not scrubbed.
        assert!(out.contains("token=abc123"));

        // No home known → text is passed through unchanged.
        assert_eq!(redact_with(line, None), line);
    }

    #[test]
    fn tail_returns_the_last_n_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.log");
        std::fs::write(&p, "l1\nl2\nl3\nl4\nl5\n").unwrap();
        assert_eq!(tail(&p, 2), "l4\nl5");
        assert_eq!(tail(&p, 99), "l1\nl2\nl3\nl4\nl5");
        assert_eq!(tail(Path::new("/no/such/file"), 5), "");
    }
}
