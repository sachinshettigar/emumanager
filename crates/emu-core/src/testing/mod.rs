//! In-memory fakes for every port, behind the `testing` cargo feature.
//!
//! These let `emu-core` orchestration logic (and downstream crates' tests) run with **no real
//! process spawning, network, filesystem, or wall clock** — the rule from `AGENTS.md` §6.4.
//!
//! - [`FakeProcessRunner`] — scripted stdout / exit codes, records every call.
//! - [`FakeDownloader`] — serves stubbed bytes from memory, does real SHA-256 verification.
//! - [`FakeClock`] — a settable, advanceable clock.
//! - [`InMemoryFs`] — a `BTreeMap`-backed filesystem.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use time::macros::datetime;
use time::{Duration, OffsetDateTime};
use url::Url;

use crate::error::{CoreError, Result};
use crate::model::job::{JobHandle, Progress};
use crate::ports::{ChildProcess, Clock, Command, Downloader, Fs, Output, ProcessRunner, Verified};

// ===========================================================================
// FakeProcessRunner
// ===========================================================================

/// A scripted [`ProcessRunner`]. Register expected outputs with [`Self::on_run`] /
/// [`Self::on_spawn`]; unmatched calls return [`CoreError::Invalid`] so tests fail loudly.
#[derive(Debug, Default)]
pub struct FakeProcessRunner {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    run_rules: VecDeque<RunRule>,
    spawn_rules: VecDeque<SpawnRule>,
    calls: Vec<Command>,
}

#[derive(Debug)]
struct RunRule {
    contains: Option<String>,
    output: Output,
}

#[derive(Debug)]
struct SpawnRule {
    contains: Option<String>,
    lines: Vec<String>,
    output: Output,
}

impl FakeProcessRunner {
    /// A runner with no rules yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Expect a `run` whose rendered command line contains `needle`; reply with `output`.
    /// Rules are consumed in registration order among those that match.
    #[must_use]
    pub fn on_run(self, needle: impl Into<String>, output: Output) -> Self {
        self.inner
            .lock()
            .expect("poisoned")
            .run_rules
            .push_back(RunRule {
                contains: Some(needle.into()),
                output,
            });
        self
    }

    /// Expect any `run`; reply with `output`.
    #[must_use]
    pub fn on_any_run(self, output: Output) -> Self {
        self.inner
            .lock()
            .expect("poisoned")
            .run_rules
            .push_back(RunRule {
                contains: None,
                output,
            });
        self
    }

    /// Expect a `spawn` matching `needle`; the child emits `lines` then exits with `output`.
    #[must_use]
    pub fn on_spawn(
        self,
        needle: impl Into<String>,
        lines: impl IntoIterator<Item = impl Into<String>>,
        output: Output,
    ) -> Self {
        self.inner
            .lock()
            .expect("poisoned")
            .spawn_rules
            .push_back(SpawnRule {
                contains: Some(needle.into()),
                lines: lines.into_iter().map(Into::into).collect(),
                output,
            });
        self
    }

    /// Every command passed to `run` or `spawn`, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<Command> {
        self.inner.lock().expect("poisoned").calls.clone()
    }

    /// Number of commands seen.
    #[must_use]
    pub fn call_count(&self) -> usize {
        self.inner.lock().expect("poisoned").calls.len()
    }
}

fn take_matching<T>(
    q: &mut VecDeque<T>,
    line: &str,
    get: impl Fn(&T) -> Option<&str>,
) -> Option<T> {
    let idx = q.iter().position(|r| match get(r) {
        Some(sub) => line.contains(sub),
        None => true,
    })?;
    q.remove(idx)
}

#[async_trait]
impl ProcessRunner for FakeProcessRunner {
    async fn run(&self, cmd: Command) -> Result<Output> {
        let mut inner = self.inner.lock().expect("poisoned");
        inner.calls.push(cmd.clone());
        let line = cmd.display();
        match take_matching(&mut inner.run_rules, &line, |r| r.contains.as_deref()) {
            Some(rule) => Ok(rule.output),
            None => Err(CoreError::invalid(
                "fake process runner",
                format!("no scripted `run` response for `{line}`"),
            )),
        }
    }

    async fn spawn(&self, cmd: Command) -> Result<Box<dyn ChildProcess>> {
        let mut inner = self.inner.lock().expect("poisoned");
        inner.calls.push(cmd.clone());
        let line = cmd.display();
        match take_matching(&mut inner.spawn_rules, &line, |r| r.contains.as_deref()) {
            Some(rule) => Ok(Box::new(FakeChild {
                lines: rule.lines.into(),
                output: Some(rule.output),
            })),
            None => Err(CoreError::invalid(
                "fake process runner",
                format!("no scripted `spawn` response for `{line}`"),
            )),
        }
    }
}

/// Child produced by [`FakeProcessRunner::spawn`].
#[derive(Debug)]
pub struct FakeChild {
    lines: VecDeque<String>,
    output: Option<Output>,
}

#[async_trait]
impl ChildProcess for FakeChild {
    fn pid(&self) -> Option<u32> {
        Some(424_242)
    }

    async fn next_line(&mut self) -> Result<Option<String>> {
        Ok(self.lines.pop_front())
    }

    async fn wait(&mut self) -> Result<Output> {
        self.lines.clear();
        self.output
            .take()
            .ok_or_else(|| CoreError::invalid("fake child", "wait() called twice"))
    }

    async fn kill(&mut self) -> Result<()> {
        self.lines.clear();
        if self.output.is_none() {
            self.output = Some(Output {
                status: -1,
                stdout: String::new(),
                stderr: "killed".into(),
            });
        }
        Ok(())
    }
}

// ===========================================================================
// FakeDownloader
// ===========================================================================

/// A [`Downloader`] that serves registered byte blobs from memory and verifies SHA-256 for real.
///
/// It does **not** write to any filesystem — pair it with [`InMemoryFs`] if a test needs the
/// downloaded bytes on disk. [`Verified::path`] echoes back the requested destination.
#[derive(Debug, Default)]
pub struct FakeDownloader {
    inner: Mutex<DlInner>,
}

#[derive(Debug, Default)]
struct DlInner {
    stubs: BTreeMap<String, Vec<u8>>,
    fetched: Vec<(String, PathBuf)>,
}

impl FakeDownloader {
    /// Empty downloader.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Make `url` resolve to `bytes`.
    #[must_use]
    pub fn stub(self, url: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.inner
            .lock()
            .expect("poisoned")
            .stubs
            .insert(url.into(), bytes.into());
        self
    }

    /// `(url, destination)` for every completed fetch, in order.
    #[must_use]
    pub fn fetched(&self) -> Vec<(String, PathBuf)> {
        self.inner.lock().expect("poisoned").fetched.clone()
    }

    /// Lower-case hex SHA-256 of `bytes` — handy for building `expected_sha256` in tests.
    #[must_use]
    pub fn sha256_hex(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        let mut s = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

#[async_trait]
impl Downloader for FakeDownloader {
    async fn fetch(
        &self,
        url: &Url,
        into: &Path,
        expected_sha256: Option<&str>,
        job: &JobHandle,
    ) -> Result<Verified> {
        let key = url.as_str().to_string();
        let bytes = {
            let inner = self.inner.lock().expect("poisoned");
            inner.stubs.get(&key).cloned()
        }
        .ok_or_else(|| CoreError::Download {
            url: key.clone(),
            detail: "no stub registered".into(),
        })?;

        let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        job.report(Progress::log(format!("GET {key}")));
        job.report(crate::ports::pct_progress(total, total));

        let sha = Self::sha256_hex(&bytes);
        if let Some(want) = expected_sha256 {
            if !want.eq_ignore_ascii_case(&sha) {
                return Err(CoreError::Download {
                    url: key,
                    detail: format!("checksum mismatch: expected {want}, got {sha}"),
                });
            }
        }

        self.inner
            .lock()
            .expect("poisoned")
            .fetched
            .push((key, into.to_path_buf()));

        Ok(Verified {
            path: into.to_path_buf(),
            sha256: sha,
            bytes: total,
        })
    }
}

// ===========================================================================
// FakeClock
// ===========================================================================

/// A [`Clock`] you can set and advance.
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<OffsetDateTime>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self {
            now: Mutex::new(datetime!(2026-01-01 00:00:00 UTC)),
        }
    }
}

impl FakeClock {
    /// A clock starting at `start`.
    #[must_use]
    pub fn new(start: OffsetDateTime) -> Self {
        Self {
            now: Mutex::new(start),
        }
    }

    /// Jump the clock to `when`.
    pub fn set(&self, when: OffsetDateTime) {
        *self.now.lock().expect("poisoned") = when;
    }

    /// Move the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let mut n = self.now.lock().expect("poisoned");
        *n += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> OffsetDateTime {
        *self.now.lock().expect("poisoned")
    }
}

// ===========================================================================
// InMemoryFs
// ===========================================================================

/// A [`Fs`] backed by in-memory maps. Paths are used verbatim (no normalization).
#[derive(Debug, Default)]
pub struct InMemoryFs {
    inner: Mutex<FsInner>,
}

#[derive(Debug, Default)]
struct FsInner {
    files: BTreeMap<PathBuf, Vec<u8>>,
    dirs: BTreeSet<PathBuf>,
}

impl InMemoryFs {
    /// An empty filesystem.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of files currently stored.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.inner.lock().expect("poisoned").files.len()
    }

    fn add_parents(dirs: &mut BTreeSet<PathBuf>, path: &Path) {
        let mut cur = path;
        while let Some(parent) = cur.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            dirs.insert(parent.to_path_buf());
            cur = parent;
        }
    }
}

#[async_trait]
impl Fs for InMemoryFs {
    async fn ensure_dir(&self, path: &Path) -> Result<()> {
        let mut inner = self.inner.lock().expect("poisoned");
        inner.dirs.insert(path.to_path_buf());
        Self::add_parents(&mut inner.dirs, path);
        Ok(())
    }

    async fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let mut inner = self.inner.lock().expect("poisoned");
        Self::add_parents(&mut inner.dirs, path);
        inner.files.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        self.inner
            .lock()
            .expect("poisoned")
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| CoreError::Fs {
                path: path.display().to_string(),
                detail: "no such file".into(),
            })
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let inner = self.inner.lock().expect("poisoned");
        if !inner.dirs.contains(path) && path.as_os_str() != "/" {
            return Err(CoreError::Fs {
                path: path.display().to_string(),
                detail: "no such directory".into(),
            });
        }
        let mut out: Vec<PathBuf> = Vec::new();
        for p in inner.files.keys().chain(inner.dirs.iter()) {
            if p.parent() == Some(path) {
                out.push(p.clone());
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let inner = self.inner.lock().expect("poisoned");
        Ok(inner.files.contains_key(path) || inner.dirs.contains(path))
    }

    async fn remove(&self, path: &Path) -> Result<()> {
        let mut inner = self.inner.lock().expect("poisoned");
        let had_file = inner.files.remove(path).is_some();
        let had_dir = inner.dirs.remove(path);
        inner.files.retain(|p, _| !p.starts_with(path));
        inner.dirs.retain(|p| !p.starts_with(path));
        if had_file || had_dir {
            Ok(())
        } else {
            Err(CoreError::Fs {
                path: path.display().to_string(),
                detail: "nothing to remove".into(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::job::JobId;

    #[tokio::test]
    async fn process_runner_scripts_and_records() {
        let runner = FakeProcessRunner::new()
            .on_run(
                "avdmanager list device",
                Output {
                    status: 0,
                    stdout: "Available devices:\n".into(),
                    stderr: String::new(),
                },
            )
            .on_any_run(Output {
                status: 1,
                stdout: String::new(),
                stderr: "boom".into(),
            });

        let out = runner
            .run(Command::new("avdmanager").arg("list").arg("device"))
            .await
            .expect("first call");
        assert!(out.success());
        assert!(out.stdout.contains("Available devices"));

        let out2 = runner
            .run(Command::new("adb").arg("devices"))
            .await
            .expect("second");
        assert_eq!(out2.status, 1);

        let err = runner.run(Command::new("sdkmanager")).await.unwrap_err();
        assert_eq!(err.code(), "invalid");
        assert_eq!(runner.call_count(), 3);
        assert_eq!(runner.calls()[0].display(), "avdmanager list device");
    }

    #[tokio::test]
    async fn child_streams_then_exits() {
        let runner = FakeProcessRunner::new().on_spawn(
            "emulator @pixel",
            ["Boot completed", "INFO: guest booted"],
            Output {
                status: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        );
        let mut child = runner
            .spawn(Command::new("emulator").arg("@pixel"))
            .await
            .expect("spawn");
        assert_eq!(child.pid(), Some(424_242));
        assert_eq!(
            child.next_line().await.unwrap().as_deref(),
            Some("Boot completed")
        );
        assert_eq!(
            child.next_line().await.unwrap().as_deref(),
            Some("INFO: guest booted")
        );
        assert_eq!(child.next_line().await.unwrap(), None);
        assert!(child.wait().await.unwrap().success());
    }

    #[tokio::test]
    async fn downloader_serves_bytes_and_verifies_checksum() {
        let body = b"cmdline-tools.zip contents";
        let url = "https://dl.google.com/android/repository/commandlinetools.zip";
        let dl = FakeDownloader::new().stub(url, body.to_vec());
        let (job, log) = JobHandle::collector(JobId("j".into()));

        let good_sha = FakeDownloader::sha256_hex(body);
        let v = dl
            .fetch(
                &Url::parse(url).unwrap(),
                Path::new("/data/commandlinetools.zip"),
                Some(&good_sha),
                &job,
            )
            .await
            .expect("fetch ok");
        assert_eq!(v.bytes, u64::try_from(body.len()).unwrap());
        assert_eq!(v.sha256, good_sha);
        assert_eq!(v.path, Path::new("/data/commandlinetools.zip"));
        assert!(!log.lock().unwrap().is_empty(), "progress was reported");

        let bad = dl
            .fetch(
                &Url::parse(url).unwrap(),
                Path::new("/x"),
                Some("deadbeef"),
                &job,
            )
            .await
            .unwrap_err();
        assert_eq!(bad.code(), "download_failed");

        let missing = dl
            .fetch(
                &Url::parse("https://example.com/nope.zip").unwrap(),
                Path::new("/x"),
                None,
                &job,
            )
            .await
            .unwrap_err();
        assert_eq!(missing.code(), "download_failed");

        assert_eq!(dl.fetched().len(), 1);
    }

    #[test]
    fn clock_sets_and_advances() {
        let clock = FakeClock::new(datetime!(2026-09-05 12:00:00 UTC));
        assert_eq!(clock.now(), datetime!(2026-09-05 12:00:00 UTC));
        clock.advance(Duration::hours(3));
        assert_eq!(clock.now(), datetime!(2026-09-05 15:00:00 UTC));
        clock.set(datetime!(2000-01-01 00:00:00 UTC));
        assert_eq!(clock.now().year(), 2000);
    }

    #[tokio::test]
    async fn in_memory_fs_basic_ops() {
        let fs = InMemoryFs::new();
        fs.ensure_dir(Path::new("/sdk/system-images"))
            .await
            .unwrap();
        fs.write_atomic(Path::new("/sdk/system-images/x/build.prop"), b"ro.build=1")
            .await
            .unwrap();

        assert!(fs
            .exists(Path::new("/sdk/system-images/x/build.prop"))
            .await
            .unwrap());
        assert!(fs.exists(Path::new("/sdk/system-images")).await.unwrap());
        assert_eq!(
            fs.read(Path::new("/sdk/system-images/x/build.prop"))
                .await
                .unwrap(),
            b"ro.build=1"
        );

        let listing = fs
            .list_dir(Path::new("/sdk/system-images/x"))
            .await
            .unwrap();
        assert_eq!(
            listing,
            vec![PathBuf::from("/sdk/system-images/x/build.prop")]
        );

        assert_eq!(
            fs.read(Path::new("/missing")).await.unwrap_err().code(),
            "fs_error"
        );

        fs.remove(Path::new("/sdk/system-images")).await.unwrap();
        assert!(!fs
            .exists(Path::new("/sdk/system-images/x/build.prop"))
            .await
            .unwrap());
        assert_eq!(fs.file_count(), 0);
    }
}
