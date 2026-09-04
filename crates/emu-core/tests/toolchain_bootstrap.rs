//! Real, `#[ignore]`d end-to-end check of `bootstrap()`: resolves today's real component catalog,
//! downloads `cmdline-tools` and `platform-tools` from Google for real, and asserts
//! `sdkmanager --version` succeeds afterward against a scratch temp dir.
//!
//! Excluded from `just validate` (per `docs/architecture.md` §6 — integration tests are
//! `--ignored`, not part of the default gate). Run explicitly:
//!
//! ```text
//! just test-integration
//! ```
//!
//! Needs real network access and a JDK 17+ already on `PATH`/`JAVA_HOME`
//! (`docs/adr/0006-require-system-jdk.md`) — this test does not install one.
//!
//! `emu-core` must never depend on `tauri` (AGENTS.md §6.1), so this can't reuse `src-tauri`'s
//! real `NativeFs`/`NativeDownloader`/`NativeProcessRunner` (task 0011) directly. The impls below
//! are small, test-local stand-ins over the same real `tokio::fs` / `reqwest` / `tokio::process` —
//! real enough to prove the real path works, without production's atomic-write/progress-throttle
//! niceties this one-shot test doesn't need.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use emu_core::model::component::{ComponentId, HostArch, HostOs};
use emu_core::model::job::{JobHandle, JobId, Progress};
use emu_core::ports::{
    ChildProcess, Command as PortCommand, Downloader, Fs, Output, ProcessRunner, Verified,
};
use emu_core::toolchain::{binary_path, bootstrap, scan, BootstrapPorts};
use emu_core::{CoreError, Result};

struct TestFs;

#[async_trait]
impl Fs for TestFs {
    async fn ensure_dir(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| fs_err(path, &e))
    }

    async fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| fs_err(parent, &e))?;
        }
        tokio::fs::write(path, bytes)
            .await
            .map_err(|e| fs_err(path, &e))
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path).await.map_err(|e| fs_err(path, &e))
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| fs_err(path, &e))?;
        while let Some(entry) = entries.next_entry().await.map_err(|e| fs_err(path, &e))? {
            out.push(entry.path());
        }
        Ok(out)
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        Ok(tokio::fs::try_exists(path).await.unwrap_or(false))
    }

    async fn remove(&self, path: &Path) -> Result<()> {
        let meta = tokio::fs::metadata(path)
            .await
            .map_err(|e| fs_err(path, &e))?;
        if meta.is_dir() {
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(|e| fs_err(path, &e))
        } else {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| fs_err(path, &e))
        }
    }
}

fn fs_err(path: &Path, e: &std::io::Error) -> CoreError {
    CoreError::Fs {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

struct TestDownloader {
    client: reqwest::Client,
}

#[async_trait]
impl Downloader for TestDownloader {
    async fn fetch(
        &self,
        url: &url::Url,
        into: &Path,
        _expected_sha256: Option<&str>,
        job: &JobHandle,
    ) -> Result<Verified> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| download_err(url, &e))?;
        let bytes = response.bytes().await.map_err(|e| download_err(url, &e))?;
        if let Some(parent) = into.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| fs_err(parent, &e))?;
        }
        tokio::fs::write(into, &bytes)
            .await
            .map_err(|e| fs_err(into, &e))?;
        job.report(Progress::log(format!(
            "downloaded {} bytes from {url}",
            bytes.len()
        )));
        Ok(Verified {
            path: into.to_path_buf(),
            // `bootstrap()` verifies SHA-1 itself against the catalog; this test downloader
            // doesn't need to also compute a SHA-256 nothing checks.
            sha256: String::new(),
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        })
    }
}

fn download_err(url: &url::Url, e: &reqwest::Error) -> CoreError {
    CoreError::Download {
        url: url.to_string(),
        detail: e.to_string(),
    }
}

struct TestProcessRunner;

#[async_trait]
impl ProcessRunner for TestProcessRunner {
    async fn run(&self, cmd: PortCommand) -> Result<Output> {
        use tokio::io::AsyncWriteExt as _;

        let mut command = tokio::process::Command::new(&cmd.program);
        command.args(&cmd.args);
        if let Some(dir) = &cmd.cwd {
            command.current_dir(dir);
        }
        for (k, v) in &cmd.env {
            command.env(k, v);
        }
        command.stdin(if cmd.stdin.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        });
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn().map_err(|e| process_err(&cmd.program, &e))?;
        if let Some(bytes) = &cmd.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(bytes).await;
            }
        }
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| process_err(&cmd.program, &e))?;
        Ok(Output {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    async fn spawn(&self, _cmd: PortCommand) -> Result<Box<dyn ChildProcess>> {
        Err(CoreError::NotImplemented(
            "TestProcessRunner::spawn — bootstrap only ever calls run()",
        ))
    }
}

fn process_err(program: &str, e: &std::io::Error) -> CoreError {
    CoreError::Process {
        program: program.to_string(),
        code: -1,
        stderr: e.to_string(),
    }
}

#[tokio::test]
#[ignore = "real network + a real multi-hundred-MB SDK download; run via `just test-integration`"]
async fn bootstrap_downloads_cmdline_tools_and_platform_tools_for_real() {
    let os = HostOs::current().expect("this test runs on a supported host OS");
    let arch = HostArch::current().expect("this test runs on a supported host arch");

    let client = reqwest::Client::new();
    let manifest_url = format!("{}repository2-3.xml", emu_android::catalog::BASE_URL);
    let xml = client
        .get(&manifest_url)
        .send()
        .await
        .expect("fetch the real repository manifest")
        .bytes()
        .await
        .expect("read the manifest body");
    let catalog = emu_android::catalog::parse(&xml, os, arch).expect("parse the real manifest");

    // Only the two components MILESTONES.md's M1 DoD asks this test to prove — `emulator` is a
    // much larger download and isn't needed to prove the bootstrap path works end to end.
    let wanted: Vec<_> = catalog
        .into_iter()
        .filter(|c| matches!(c.id, ComponentId::CmdlineTools | ComponentId::PlatformTools))
        .collect();
    assert_eq!(
        wanted.len(),
        2,
        "expected both cmdline-tools and platform-tools in today's real catalog for this host"
    );

    let data_dir = tempfile::tempdir().expect("scratch temp dir");
    let app_sdk_dir = data_dir.path().join("sdk");

    let fs = TestFs;
    let downloader = TestDownloader {
        client: reqwest::Client::new(),
    };
    let process = TestProcessRunner;
    let ports = BootstrapPorts {
        fs: &fs,
        downloader: &downloader,
        process: &process,
    };

    let state = scan(&fs, &app_sdk_dir, os, |k| std::env::var(k).ok())
        .await
        .expect("scan a fresh, empty dir");
    assert!(
        state
            .missing(&[ComponentId::CmdlineTools, ComponentId::PlatformTools])
            .len()
            == 2,
        "a freshly created temp dir must not already look bootstrapped"
    );

    let (job, _log) = JobHandle::collector(JobId("toolchain-bootstrap-integration-test".into()));
    bootstrap(data_dir.path(), &wanted, &state, os, &ports, &job)
        .await
        .expect("real bootstrap succeeds");

    let sdkmanager = binary_path(&app_sdk_dir, ComponentId::CmdlineTools, os);
    let version = process
        .run(PortCommand::new(sdkmanager.display().to_string()).arg("--version"))
        .await
        .expect("run real sdkmanager --version");
    assert!(
        version.success(),
        "sdkmanager --version failed: {}",
        version.stderr
    );
    assert!(
        !version.stdout.trim().is_empty(),
        "sdkmanager --version printed nothing"
    );
}
