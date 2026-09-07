//! `bootstrap()` — turns an empty (or partially-populated) data dir into a working, licensed
//! `sdkmanager` with everything in `wanted` installed.
//!
//! Only `cmdline-tools;latest` is ever downloaded and unpacked by this module directly — it's the
//! one component needed before `sdkmanager` itself exists to do anything. Every other component
//! (`platform-tools`, `emulator`, and anything else later milestones add) is installed by asking
//! the real `sdkmanager` to do it (`sdkmanager "platform-tools" "emulator"`), the same way a human
//! would — `sdkmanager` already knows how to resolve, download, and verify its own packages
//! correctly; re-implementing that for every component would be needless surface area.
//!
//! Steps, each skipped when already satisfied:
//! 1. Bail out early if nothing in `wanted` is missing (checked against `state`, which already
//!    folded in any existing *system* SDK — see [`super::installed_state`]) — a true no-op.
//! 2. A system JDK 17+ check (`java -version`) — see `docs/adr/0006-require-system-jdk.md`. Only
//!    reached once we know real work is needed.
//! 3. If `cmdline-tools` is missing: download it (SHA-1-verified against the catalog, task 0010)
//!    and unpack it into `<data_dir>/sdk/cmdline-tools/latest`.
//! 4. `sdkmanager --licenses`, feeding enough `y` answers to accept every current license.
//! 5. `sdkmanager <remaining package paths...>` for whatever in `wanted` is still missing.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::error::{CoreError, Result};
use crate::model::component::{Component, ComponentId, HostOs};
use crate::model::job::{JobHandle, Progress};
use crate::ports::{Command, Downloader, Fs, Output, ProcessRunner};

use super::installed_state::{self, InstalledState};

/// The leaf ports `bootstrap` needs, bundled by reference. A plain struct, not a new trait —
/// `bootstrap` is a free function over the existing four ports, same as everything else here.
pub struct BootstrapPorts<'a> {
    /// Filesystem access, for the app-managed `sdk/` dir.
    pub fs: &'a dyn Fs,
    /// Fetches the `cmdline-tools` archive.
    pub downloader: &'a dyn Downloader,
    /// Runs `java`, `sdkmanager`, and (on Unix) `chmod`.
    pub process: &'a dyn ProcessRunner,
}

/// More `y` answers than any real `sdkmanager --licenses` session has ever needed — a live
/// capture against a real `cmdline-tools` 19.0 on 2026-09-05 needed exactly 7 (see this task's
/// Notes for the full transcript) — but still a bounded number, unlike piping `yes` forever.
/// Revisit if Google ever adds enough new licenses to approach this.
const LICENSE_ACCEPT_COUNT: usize = 50;

/// Ensure every component in `wanted` ends up installed, downloading/installing only what
/// `state` doesn't already report present (app-managed *or* system — see
/// [`super::installed_state::scan`]).
pub async fn bootstrap(
    data_dir: &Path,
    wanted: &[Component],
    state: &InstalledState,
    os: HostOs,
    ports: &BootstrapPorts<'_>,
    job: &JobHandle,
) -> Result<()> {
    let missing: Vec<&Component> = wanted
        .iter()
        .filter(|c| !state.is_installed(c.id))
        .collect();
    if missing.is_empty() {
        job.report(Progress::log(
            "everything requested is already installed; nothing to do",
        ));
        return Ok(());
    }

    ensure_jdk(ports.process).await?;

    let app_sdk_dir = data_dir.join("sdk");
    ports.fs.ensure_dir(&app_sdk_dir).await?;

    let cmdline_tools_missing = missing
        .iter()
        .find(|c| c.id == ComponentId::CmdlineTools)
        .copied();
    let just_installed_cmdline_tools = if let Some(component) = cmdline_tools_missing {
        job.report(Progress::log(format!(
            "downloading {} {}",
            component.id.repo_path(),
            component.version
        )));
        fetch_and_extract_cmdline_tools(component, &app_sdk_dir, ports, job).await?;
        true
    } else {
        false
    };

    let sdkmanager = sdkmanager_path(state, &app_sdk_dir, os, just_installed_cmdline_tools);

    let remaining_pkgs: Vec<&str> = missing
        .iter()
        .filter(|c| c.id != ComponentId::CmdlineTools)
        .map(|c| c.id.repo_path())
        .collect();

    // A license must be accepted before `sdkmanager` installs anything — needed whether we just
    // unpacked a brand-new `cmdline-tools` or are reusing one `state` already found.
    accept_licenses(&sdkmanager, ports.process, job).await?;

    if !remaining_pkgs.is_empty() {
        install_packages(&sdkmanager, &remaining_pkgs, ports.process, job).await?;
    }

    Ok(())
}

/// The `sdkmanager` entry point to invoke: under the SDK root `cmdline-tools` was found at
/// (app-managed if just installed or previously found there; whichever system root `state` says
/// otherwise).
fn sdkmanager_path(
    state: &InstalledState,
    app_sdk_dir: &Path,
    os: HostOs,
    just_installed_cmdline_tools: bool,
) -> PathBuf {
    let root = if just_installed_cmdline_tools {
        app_sdk_dir.to_path_buf()
    } else {
        state
            .location_of(ComponentId::CmdlineTools)
            .map_or_else(|| app_sdk_dir.to_path_buf(), |l| l.sdk_root.clone())
    };
    root.join(installed_state::marker_file(ComponentId::CmdlineTools, os))
}

/// A system JDK 17+ is required — `cmdline-tools` is a `java` wrapper script, not a standalone
/// binary, and Google stopped shipping one to bundle (`docs/adr/0006-require-system-jdk.md`).
/// Checked with `java -version` (real OpenJDK/Oracle builds print the version line to **stderr**,
/// not stdout — verified against a real JDK 21 install, 2026-09-05) rather than guessed.
async fn ensure_jdk(process: &dyn ProcessRunner) -> Result<()> {
    let output = process
        .run(Command::new("java").arg("-version"))
        .await
        .map_err(|_| jdk_missing_err())?;
    let text = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    match parse_java_major_version(text) {
        Some(major) if major >= 17 => Ok(()),
        Some(major) => Err(CoreError::Unsupported(format!(
            "found a JDK, but it's version {major}; Emulator Studio needs JDK 17 or newer on PATH or \
             JAVA_HOME (docs/adr/0006-require-system-jdk.md)"
        ))),
        None => Err(jdk_missing_err()),
    }
}

fn jdk_missing_err() -> CoreError {
    CoreError::Unsupported(
        "no JDK 17+ found on PATH or JAVA_HOME; install one (e.g. Eclipse Temurin 17+) and set \
         JAVA_HOME, then retry — Emulator Studio does not bundle a JRE \
         (docs/adr/0006-require-system-jdk.md)"
            .to_string(),
    )
}

/// Parse the major version out of `java -version`'s `"..."`-quoted version string. Handles both
/// the pre-JDK9 scheme (`"1.8.0_372"` = Java 8) and the JEP 223 scheme (`"17.0.9"`, `"21"`).
fn parse_java_major_version(text: &str) -> Option<u32> {
    let start = text.find('"')? + 1;
    let rest = &text[start..];
    let end = rest.find('"')?;
    let ver = &rest[..end];
    let mut segments = ver.split(['.', '_']);
    let first: u32 = segments.next()?.parse().ok()?;
    if first == 1 {
        segments.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Download `cmdline-tools;latest`, verify it against the catalog's SHA-1 (task 0010 — Google's
/// manifest doesn't publish SHA-256, so `Downloader::fetch`'s own checksum check isn't used here;
/// `expected_sha256` is passed as `None` and verification happens against the read-back bytes
/// instead), and unpack it.
async fn fetch_and_extract_cmdline_tools(
    component: &Component,
    app_sdk_dir: &Path,
    ports: &BootstrapPorts<'_>,
    job: &JobHandle,
) -> Result<()> {
    let downloads_dir = app_sdk_dir.join(".downloads");
    ports.fs.ensure_dir(&downloads_dir).await?;
    let zip_path = downloads_dir.join("cmdline-tools.zip");

    ports
        .downloader
        .fetch(&component.url, &zip_path, None, job)
        .await?;

    let bytes = ports.fs.read(&zip_path).await?;
    let got_sha1 = hex_sha1(&bytes);
    if !got_sha1.eq_ignore_ascii_case(&component.sha1) {
        let _ = ports.fs.remove(&zip_path).await;
        return Err(CoreError::Download {
            url: component.url.to_string(),
            detail: format!(
                "sha1 mismatch: catalog says {}, downloaded file is {got_sha1}",
                component.sha1
            ),
        });
    }

    job.report(Progress::log("extracting cmdline-tools"));
    extract_cmdline_tools(&bytes, app_sdk_dir, ports).await?;
    let _ = ports.fs.remove(&zip_path).await;
    Ok(())
}

/// One file or directory read out of the archive, fully owned — no `zip`-crate type survives past
/// [`read_cmdline_tools_zip`].
struct ExtractedEntry {
    /// Path relative to `cmdline-tools/latest/` (the archive's own leading segment stripped).
    relative: PathBuf,
    is_dir: bool,
    contents: Vec<u8>,
}

/// Read every entry out of the archive into memory, synchronously, with no `.await` anywhere in
/// this function.
///
/// This has to be a separate, non-`async` step from [`extract_cmdline_tools`]: `zip::ZipFile`
/// holds a `&mut dyn Read` with no `Send` bound, so a `ZipArchive`/`ZipFile` value can never be
/// held across an `.await` point in code that must produce a `Send` future — which a real Tauri
/// command's async body must (discovered when task 0013 first called this from a real command;
/// `emu-core`'s own single-threaded test runtime never required `Send` and so never caught it).
/// Splitting the zip-crate-touching code out into a plain function that returns fully owned,
/// `Send`-safe data before any `Fs` call starts fixes it at the root instead of fighting the
/// borrow checker inside one giant async loop.
///
/// The archive's own top-level folder is named `cmdline-tools/` (verified against the real
/// archive while building this task — see the task file's Notes) and must be *renamed* to
/// `latest/`: that placement is an Android convention the zip itself does not encode, so the
/// leading path segment is stripped here rather than trusted from the archive.
fn read_cmdline_tools_zip(zip_bytes: &[u8]) -> Result<Vec<ExtractedEntry>> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| extract_err(&e))?;
    let mut entries = Vec::with_capacity(archive.len());

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| extract_err(&e))?;
        let Some(name) = entry.enclosed_name() else {
            continue; // unsafe path (absolute / traversal) in the archive — never trust it
        };
        let mut components = name.components();
        components.next(); // drop the archive's own leading `cmdline-tools/` segment
        let relative = components.as_path();
        if relative.as_os_str().is_empty() {
            continue; // the top-level directory entry itself
        }
        let relative = relative.to_path_buf();

        if entry.is_dir() {
            entries.push(ExtractedEntry {
                relative,
                is_dir: true,
                contents: Vec::new(),
            });
            continue;
        }
        let mut contents = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
        entry
            .read_to_end(&mut contents)
            .map_err(|e| fs_io_err(&relative, &e))?;
        entries.push(ExtractedEntry {
            relative,
            is_dir: false,
            contents,
        });
    }
    Ok(entries)
}

/// Unpack `cmdline-tools`'s zip into `<app_sdk_dir>/cmdline-tools/latest`, entirely through the
/// `Fs` port (so this is exercisable with `InMemoryFs` in fake-driven tests, no bypass to real
/// `std::fs`).
///
/// `Fs::write_atomic` doesn't carry a permission mode, so the extracted files land without the
/// executable bit; [`make_bin_executable`] fixes that up afterward via `chmod` (a real port call,
/// not a new one) rather than bypassing `Fs` to set permissions directly.
async fn extract_cmdline_tools(
    zip_bytes: &[u8],
    app_sdk_dir: &Path,
    ports: &BootstrapPorts<'_>,
) -> Result<()> {
    // Real archives run ~140 MB of synchronous deflate decompression — `spawn_blocking` keeps
    // that off the async executor's worker thread, not just off the (already required) `Send`
    // path. `zip_bytes` is copied once into the blocking task rather than borrowed, since a
    // `'static` future can't hold a borrow of the caller's stack.
    let owned_bytes = zip_bytes.to_vec();
    let entries = tokio::task::spawn_blocking(move || read_cmdline_tools_zip(&owned_bytes))
        .await
        .map_err(|e| CoreError::Invalid {
            what: "cmdline-tools archive",
            detail: format!("extraction task panicked: {e}"),
        })??;

    let dest_root = app_sdk_dir.join("cmdline-tools/latest");
    for entry in entries {
        let out_path = dest_root.join(&entry.relative);
        if entry.is_dir {
            ports.fs.ensure_dir(&out_path).await?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            ports.fs.ensure_dir(parent).await?;
        }
        ports.fs.write_atomic(&out_path, &entry.contents).await?;
    }

    make_bin_executable(&dest_root, ports.process).await
}

/// `chmod -R +x` on the extracted `bin/` directory. A no-op on Windows, where executability comes
/// from the file extension, not a permission bit — `chmod` isn't even a real binary there.
async fn make_bin_executable(dest_root: &Path, process: &dyn ProcessRunner) -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    let bin_dir = dest_root.join("bin");
    let output = process
        .run(
            Command::new("chmod")
                .arg("-R")
                .arg("+x")
                .arg(bin_dir.display().to_string()),
        )
        .await?;
    if output.success() {
        Ok(())
    } else {
        Err(CoreError::Process {
            program: "chmod".to_string(),
            code: output.status,
            stderr: output.stderr,
        })
    }
}

/// Run `cmd` to completion, forwarding every output line to `job` as it arrives (rather than
/// dumping them all at the end once the process exits). `sdkmanager`'s big downloads —
/// `emulator` is ~900 MB — otherwise look frozen for minutes. No assumption is made about the
/// *shape* of a line: it's `sdkmanager`'s own text, passed straight through as a log line.
async fn run_streamed(
    cmd: Command,
    process: &dyn ProcessRunner,
    job: &JobHandle,
) -> Result<Output> {
    let mut child = process.spawn(cmd).await?;
    while let Some(line) = child.next_line().await? {
        let trimmed = line.trim_end();
        if !trimmed.is_empty() {
            job.report(Progress::log(trimmed.to_string()));
        }
    }
    child.wait().await
}

fn process_err(sdkmanager: &Path, output: &Output) -> CoreError {
    CoreError::Process {
        program: sdkmanager.display().to_string(),
        code: output.status,
        stderr: output.stderr.clone(),
    }
}

/// `sdkmanager --licenses`, answering every prompt `y`. Real, captured behavior of `sdkmanager`
/// 19.0 (`cmdline-tools` resolved by task 0010's catalog, run live on 2026-09-05): it prints
/// `"N of N SDK package licenses not accepted."`, then for each one in turn the license text
/// followed by a literal `Accept? (y/N): ` prompt reading one line of stdin, and finally `"All SDK
/// package licenses accepted"` once every prompt has been answered. Feeding
/// [`LICENSE_ACCEPT_COUNT`] `y` answers up front and closing stdin (the real capture needed 7)
/// accepts them all in one non-interactive call — the documented `yes | sdkmanager --licenses`
/// pattern, bounded instead of infinite.
async fn accept_licenses(
    sdkmanager: &Path,
    process: &dyn ProcessRunner,
    job: &JobHandle,
) -> Result<()> {
    job.report(Progress {
        phase: Some("Accepting SDK licenses".to_string()),
        ..Progress::empty()
    });
    let stdin = "y\n".repeat(LICENSE_ACCEPT_COUNT).into_bytes();
    let cmd = Command {
        stdin: Some(stdin),
        ..Command::new(sdkmanager.display().to_string()).arg("--licenses")
    };
    let output = run_streamed(cmd, process, job).await?;
    if output.success() {
        Ok(())
    } else {
        Err(process_err(sdkmanager, &output))
    }
}

/// `sdkmanager <packages...>` — one call installing everything still missing besides
/// `cmdline-tools` itself. `sdkmanager` accepts any number of package paths in one invocation.
/// Output is streamed to `job` line by line as `sdkmanager` downloads and unpacks.
async fn install_packages(
    sdkmanager: &Path,
    packages: &[&str],
    process: &dyn ProcessRunner,
    job: &JobHandle,
) -> Result<()> {
    job.report(Progress {
        phase: Some(format!("Downloading & installing {}", packages.join(", "))),
        ..Progress::empty()
    });
    let cmd = Command::new(sdkmanager.display().to_string()).args(packages.iter().copied());
    let output = run_streamed(cmd, process, job).await?;
    if output.success() {
        Ok(())
    } else {
        Err(process_err(sdkmanager, &output))
    }
}

fn hex_sha1(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    let mut s = String::with_capacity(40);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn extract_err(e: &zip::result::ZipError) -> CoreError {
    CoreError::Invalid {
        what: "cmdline-tools archive",
        detail: e.to_string(),
    }
}

fn fs_io_err(path: &Path, e: &std::io::Error) -> CoreError {
    CoreError::Fs {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;
    use crate::model::job::JobId;
    use crate::ports::Output;
    use crate::testing::{FakeDownloader, FakeProcessRunner, InMemoryFs};

    /// A tiny but real zip, shaped like the real archive: one top-level `cmdline-tools/` folder
    /// containing `bin/sdkmanager`. Exercises the exact same extraction path a real ~140 MB
    /// archive would, just with a few bytes of fake script content instead of a real jar-backed
    /// launcher.
    fn tiny_cmdline_tools_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("cmdline-tools/bin/sdkmanager", opts).unwrap();
            w.write_all(b"#!/bin/sh\necho fake sdkmanager\n").unwrap();
            w.start_file("cmdline-tools/source.properties", opts)
                .unwrap();
            w.write_all(b"Pkg.Revision=19.0\n").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    fn ok(stdout: impl Into<String>) -> Output {
        Output {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    fn jdk_ok() -> Output {
        Output {
            status: 0,
            stdout: String::new(),
            stderr: "openjdk version \"21\" 2023-09-19 LTS".into(),
        }
    }

    #[tokio::test]
    async fn fresh_bootstrap_downloads_licenses_and_installs() {
        let zip_bytes = tiny_cmdline_tools_zip();
        let sha1 = hex_sha1(&zip_bytes);
        let url = "https://dl.google.com/android/repository/commandlinetools-fake.zip";

        let cmdline_tools = Component {
            id: ComponentId::CmdlineTools,
            version: "19.0".into(),
            url: url.parse().unwrap(),
            size_bytes: u64::try_from(zip_bytes.len()).unwrap(),
            sha1,
        };
        let platform_tools = Component {
            id: ComponentId::PlatformTools,
            version: "36.0.0".into(),
            url: "https://dl.google.com/android/repository/platform-tools_r36-linux.zip"
                .parse()
                .unwrap(),
            size_bytes: 0,
            sha1: "unused-installed-via-sdkmanager".into(),
        };

        let fs = InMemoryFs::new();
        // What a real `Downloader::fetch` would already have written before bootstrap reads it
        // back — `FakeDownloader` deliberately doesn't touch the filesystem itself.
        fs.write_atomic(
            Path::new("/data/sdk/.downloads/cmdline-tools.zip"),
            &zip_bytes,
        )
        .await
        .unwrap();

        let downloader = FakeDownloader::new().stub(url, zip_bytes.clone());
        let process = FakeProcessRunner::new()
            .on_run("java -version", jdk_ok())
            .on_run("chmod -R +x", ok(""))
            .on_spawn(
                "sdkmanager --licenses",
                [
                    "7 of 7 SDK package licenses not accepted",
                    "All SDK package licenses accepted",
                ],
                ok(""),
            )
            .on_spawn(
                "sdkmanager platform-tools",
                [
                    "Preparing \"Install Android SDK Platform-Tools\"",
                    "\"Install Android SDK Platform-Tools\" (revision: 36.0.0): 100%",
                ],
                ok(""),
            );
        let ports = BootstrapPorts {
            fs: &fs,
            downloader: &downloader,
            process: &process,
        };
        let (job, log) = JobHandle::collector(JobId("j".into()));

        bootstrap(
            Path::new("/data"),
            &[cmdline_tools, platform_tools],
            &InstalledState::default(),
            HostOs::Linux,
            &ports,
            &job,
        )
        .await
        .expect("bootstrap succeeds");

        // `sdkmanager`'s own output lines are forwarded to the job as they arrive, not buffered.
        let (saw_install_line, saw_license_phase) = {
            let reported = log.lock().unwrap();
            (
                reported.iter().any(|p| {
                    p.log_line
                        .as_deref()
                        .is_some_and(|l| l.contains("Install Android SDK Platform-Tools"))
                }),
                reported
                    .iter()
                    .any(|p| p.phase.as_deref() == Some("Accepting SDK licenses")),
            )
        };
        assert!(
            saw_install_line,
            "sdkmanager output lines are forwarded live"
        );
        assert!(saw_license_phase, "the license phase is reported");

        assert!(fs
            .exists(Path::new("/data/sdk/cmdline-tools/latest/bin/sdkmanager"))
            .await
            .unwrap());
        assert_eq!(
            fs.read(Path::new("/data/sdk/cmdline-tools/latest/bin/sdkmanager"))
                .await
                .unwrap(),
            b"#!/bin/sh\necho fake sdkmanager\n"
        );

        let calls = process.calls();
        let licenses_call = calls
            .iter()
            .find(|c| c.display().contains("--licenses"))
            .expect("licenses call happened");
        assert_eq!(
            licenses_call.stdin.as_deref(),
            Some("y\n".repeat(LICENSE_ACCEPT_COUNT).as_bytes())
        );
    }

    #[tokio::test]
    async fn already_bootstrapped_is_a_no_op() {
        use crate::toolchain::installed_state::{ComponentLocation, SdkSource};

        let cmdline_tools = Component {
            id: ComponentId::CmdlineTools,
            version: "19.0".into(),
            url: "https://dl.google.com/x/cmdline-tools.zip".parse().unwrap(),
            size_bytes: 0,
            sha1: String::new(),
        };
        let platform_tools = Component {
            id: ComponentId::PlatformTools,
            version: "36.0.0".into(),
            url: "https://dl.google.com/x/platform-tools.zip"
                .parse()
                .unwrap(),
            size_bytes: 0,
            sha1: String::new(),
        };
        let state = InstalledState::from_found(vec![
            ComponentLocation {
                id: ComponentId::CmdlineTools,
                sdk_root: PathBuf::from("/data/sdk"),
                source: SdkSource::AppManaged,
            },
            ComponentLocation {
                id: ComponentId::PlatformTools,
                sdk_root: PathBuf::from("/data/sdk"),
                source: SdkSource::AppManaged,
            },
        ]);

        let fs = InMemoryFs::new();
        let downloader = FakeDownloader::new();
        // No rules registered: any `run`/`spawn` call fails the test loudly.
        let process = FakeProcessRunner::new();
        let ports = BootstrapPorts {
            fs: &fs,
            downloader: &downloader,
            process: &process,
        };
        let (job, log) = JobHandle::collector(JobId("j".into()));

        bootstrap(
            Path::new("/data"),
            &[cmdline_tools, platform_tools],
            &state,
            HostOs::Linux,
            &ports,
            &job,
        )
        .await
        .expect("no-op bootstrap succeeds");

        assert_eq!(process.call_count(), 0);
        assert!(downloader.fetched().is_empty());
        assert!(log.lock().unwrap().iter().any(|p| p.log_line.as_deref()
            == Some("everything requested is already installed; nothing to do")));
    }

    #[tokio::test]
    async fn failed_download_surfaces_the_real_error() {
        let cmdline_tools = Component {
            id: ComponentId::CmdlineTools,
            version: "19.0".into(),
            url: "https://dl.google.com/x/cmdline-tools.zip".parse().unwrap(),
            size_bytes: 0,
            sha1: "deadbeef".into(),
        };
        let fs = InMemoryFs::new();
        let downloader = FakeDownloader::new(); // no stub registered for the URL
        let process = FakeProcessRunner::new().on_run("java -version", jdk_ok());
        let ports = BootstrapPorts {
            fs: &fs,
            downloader: &downloader,
            process: &process,
        };
        let (job, _log) = JobHandle::collector(JobId("j".into()));

        let err = bootstrap(
            Path::new("/data"),
            &[cmdline_tools],
            &InstalledState::default(),
            HostOs::Linux,
            &ports,
            &job,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "download_failed");
    }

    #[tokio::test]
    async fn missing_jdk_fails_fast_before_any_download() {
        let cmdline_tools = Component {
            id: ComponentId::CmdlineTools,
            version: "19.0".into(),
            url: "https://dl.google.com/x/cmdline-tools.zip".parse().unwrap(),
            size_bytes: 0,
            sha1: String::new(),
        };
        let fs = InMemoryFs::new();
        let downloader = FakeDownloader::new();
        // `run` for "java -version" has no rule -> FakeProcessRunner errors -> ensure_jdk maps it
        // to the "no JDK found" error, same as a real "command not found".
        let process = FakeProcessRunner::new();
        let ports = BootstrapPorts {
            fs: &fs,
            downloader: &downloader,
            process: &process,
        };
        let (job, _log) = JobHandle::collector(JobId("j".into()));

        let err = bootstrap(
            Path::new("/data"),
            &[cmdline_tools],
            &InstalledState::default(),
            HostOs::Linux,
            &ports,
            &job,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "unsupported");
        assert!(downloader.fetched().is_empty());
    }

    #[test]
    fn java_version_parsing_handles_both_schemes() {
        assert_eq!(
            parse_java_major_version("openjdk version \"21\" 2023-09-19"),
            Some(21)
        );
        assert_eq!(
            parse_java_major_version("java version \"17.0.9\" 2023-10-17"),
            Some(17)
        );
        assert_eq!(
            parse_java_major_version("java version \"1.8.0_372\""),
            Some(8)
        );
        assert_eq!(parse_java_major_version("not a version string"), None);
    }
}
