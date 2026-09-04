//! Ports — the traits `emu-core` calls out through for every side effect.
//!
//! Real implementations live in `emu-android` / `emu-host` / `src-tauri`; the `testing` feature
//! ships in-memory fakes (see [`crate::testing`]). All traits are `Send + Sync` and object-safe
//! so they can be held as `Arc<dyn Port>`. Async methods use [`mod@async_trait`] for dyn-safety.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use time::OffsetDateTime;
use url::Url;

use crate::error::Result;
use crate::model::job::{JobHandle, Progress};

// ---------------------------------------------------------------------------
// Process execution
// ---------------------------------------------------------------------------

/// A command to run: program plus arguments and environment. No shell involved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    /// Executable name or path (e.g. `avdmanager`).
    pub program: String,
    /// Arguments, already split (no shell quoting).
    pub args: Vec<String>,
    /// Working directory; `None` inherits the parent's.
    pub cwd: Option<PathBuf>,
    /// Extra environment entries layered on top of the parent environment.
    pub env: Vec<(String, String)>,
    /// Bytes to write to the child's stdin before closing it.
    pub stdin: Option<Vec<u8>>,
}

impl Command {
    /// Start building a command from its program name.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            ..Self::default()
        }
    }

    /// Append one argument.
    #[must_use]
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    /// Append several arguments.
    #[must_use]
    pub fn args<I, S>(mut self, it: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(it.into_iter().map(Into::into));
        self
    }

    /// Set the working directory.
    #[must_use]
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.env.push((k.into(), v.into()));
        self
    }

    /// Render as a display string (for logs). Not shell-safe; do not execute it.
    #[must_use]
    pub fn display(&self) -> String {
        let mut s = self.program.clone();
        for a in &self.args {
            s.push(' ');
            s.push_str(a);
        }
        s
    }
}

/// The result of a finished process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Exit code (`-1` if the process was killed by a signal).
    pub status: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl Output {
    /// `true` when the process exited `0`.
    #[must_use]
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

/// A still-running child process.
#[async_trait]
pub trait ChildProcess: Send {
    /// OS process id, if known.
    fn pid(&self) -> Option<u32>;

    /// Read the next line from the merged stdout/stderr stream, or `None` at EOF.
    async fn next_line(&mut self) -> Result<Option<String>>;

    /// Wait for the process to exit and collect any remaining output.
    async fn wait(&mut self) -> Result<Output>;

    /// Ask the process to terminate.
    async fn kill(&mut self) -> Result<()>;
}

/// Runs external programs (`sdkmanager`, `avdmanager`, `emulator`, `adb`).
#[async_trait]
pub trait ProcessRunner: Send + Sync {
    /// Run to completion, capturing all output.
    async fn run(&self, cmd: Command) -> Result<Output>;

    /// Spawn and return a handle for streaming/long-lived processes (the emulator).
    async fn spawn(&self, cmd: Command) -> Result<Box<dyn ChildProcess>>;
}

// ---------------------------------------------------------------------------
// Downloads
// ---------------------------------------------------------------------------

/// A completed, checksum-verified download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// Where the file landed.
    pub path: PathBuf,
    /// Lower-case hex SHA-256 of the file.
    pub sha256: String,
    /// Size in bytes.
    pub bytes: u64,
}

/// Fetches files over HTTP(S) with progress and SHA-256 verification.
#[async_trait]
pub trait Downloader: Send + Sync {
    /// Download `url` to `into`, reporting progress on `job`.
    ///
    /// If `expected_sha256` is `Some`, the download fails unless the hash matches.
    async fn fetch(
        &self,
        url: &Url,
        into: &Path,
        expected_sha256: Option<&str>,
        job: &JobHandle,
    ) -> Result<Verified>;
}

// ---------------------------------------------------------------------------
// Host probe
// ---------------------------------------------------------------------------

/// Inspects the machine for emulator readiness (virtualization, accelerator, disk, RAM).
#[async_trait]
pub trait HostProbe: Send + Sync {
    /// Produce a fresh [`HostReport`](crate::model::host::HostReport).
    async fn inspect(&self) -> Result<crate::model::host::HostReport>;
}

// ---------------------------------------------------------------------------
// Clock
// ---------------------------------------------------------------------------

/// The current time. Injected so timestamps are deterministic in tests.
pub trait Clock: Send + Sync {
    /// Now, as a UTC-offset datetime.
    fn now(&self) -> OffsetDateTime;
}

// ---------------------------------------------------------------------------
// Filesystem
// ---------------------------------------------------------------------------

/// Filesystem operations `emu-core` needs, abstracted for tests and atomicity guarantees.
#[async_trait]
pub trait Fs: Send + Sync {
    /// Create `path` and any missing parents. No error if it already exists.
    async fn ensure_dir(&self, path: &Path) -> Result<()>;

    /// Write `bytes` to `path` atomically (temp file + rename).
    async fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()>;

    /// Read the whole file at `path`.
    async fn read(&self, path: &Path) -> Result<Vec<u8>>;

    /// List the immediate entries of directory `path` (full paths, unsorted).
    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Whether `path` exists (file or directory).
    async fn exists(&self, path: &Path) -> Result<bool>;

    /// Remove a file or an empty/​non-empty directory tree at `path`.
    async fn remove(&self, path: &Path) -> Result<()>;
}

/// Convenience: report a download-style [`Progress`] with a percentage.
#[must_use]
pub fn pct_progress(done: u64, total: u64) -> Progress {
    let pct = done
        .saturating_mul(100)
        .checked_div(total)
        .map(|p| u8::try_from(p).unwrap_or(100));
    Progress {
        bytes_done: Some(done),
        bytes_total: Some(total),
        pct,
        ..Progress::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_builder_shapes_argv() {
        let c = Command::new("avdmanager")
            .arg("create")
            .arg("avd")
            .args(["-n", "pixel6"])
            .env("ANDROID_SDK_ROOT", "/sdk")
            .current_dir("/tmp");
        assert_eq!(c.program, "avdmanager");
        assert_eq!(c.args, ["create", "avd", "-n", "pixel6"]);
        assert_eq!(c.display(), "avdmanager create avd -n pixel6");
        assert_eq!(c.cwd.as_deref(), Some(Path::new("/tmp")));
        assert_eq!(
            c.env,
            [("ANDROID_SDK_ROOT".to_string(), "/sdk".to_string())]
        );
    }

    #[test]
    fn output_success_predicate() {
        assert!(Output {
            status: 0,
            stdout: String::new(),
            stderr: String::new()
        }
        .success());
        assert!(!Output {
            status: 1,
            stdout: String::new(),
            stderr: String::new()
        }
        .success());
    }

    #[test]
    fn pct_progress_handles_zero_total() {
        assert_eq!(pct_progress(0, 0).pct, None);
        assert_eq!(pct_progress(50, 200).pct, Some(25));
        assert_eq!(pct_progress(999, 100).pct, Some(100));
    }
}
