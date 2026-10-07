//! JDK discovery and managed download — `sdkmanager`/`avdmanager` are `java` launchers, so a JDK
//! 17+ must exist before anything else can happen. Nothing is assumed about the machine:
//!
//! 1. **Reuse** whatever is already there, in this order: the JDK Emulator Studio itself resolved
//!    last time, `JAVA_HOME`, `java` on `PATH`, then well-known install locations (Homebrew,
//!    `/usr/lib/jvm`, Adoptium/Microsoft/Zulu folders, the `~/.jdks` folder, Android Studio's
//!    bundled JBR, SDKMAN). Only a JDK that actually answers `java -version` with 17+ counts.
//! 2. **Otherwise download** Eclipse Temurin 17 into `<data_dir>/jdk/temurin-17` (checksum-verified
//!    against the Adoptium API's own SHA-256) — see `docs/adr/0008-resolve-or-download-jdk.md`.
//!
//! Whatever is chosen is recorded in `<data_dir>/jdk/java_home`, and every `sdkmanager` /
//! `avdmanager` invocation gets `JAVA_HOME` set to it ([`java_env`]), so a JDK found in a
//! non-`PATH` place (or a stale/old `JAVA_HOME` in the user's shell) never breaks the tools.

use std::path::{Path, PathBuf};

use url::Url;

use crate::error::{CoreError, Result};
use crate::model::component::{HostArch, HostOs};
use crate::model::job::{JobHandle, Progress};
use crate::ports::{Command, Fs};

use super::bootstrap::{read_zip_stripping_top_dir, BootstrapPorts};

/// Oldest JDK `sdkmanager` / `avdmanager` run on.
pub const MIN_JDK_MAJOR: u32 = 17;

/// The Temurin feature release downloaded when the machine has no usable JDK.
const MANAGED_MAJOR: u32 = 17;

/// Where a usable JDK was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JdkOrigin {
    /// Recorded by a previous run (an earlier discovery or our own download).
    Remembered,
    /// The `JAVA_HOME` environment variable.
    JavaHomeEnv,
    /// `java` on `PATH`.
    Path,
    /// A well-known install location (Android Studio's JBR, Homebrew, `/usr/lib/jvm`, …).
    WellKnown,
    /// Downloaded by Emulator Studio.
    Downloaded,
}

/// A JDK that answered `java -version` with [`MIN_JDK_MAJOR`] or newer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jdk {
    /// The JDK's home directory (what `JAVA_HOME` should point at); `None` only when `java` on
    /// `PATH` works but won't say where it lives.
    pub java_home: Option<PathBuf>,
    /// Major version (`17`, `21`, …).
    pub major: u32,
    /// How it was found.
    pub origin: JdkOrigin,
}

/// File recording the JDK home in use (one line, the path).
#[must_use]
pub fn marker_path(data_dir: &Path) -> PathBuf {
    data_dir.join("jdk").join("java_home")
}

fn java_binary(home: &Path, os: HostOs) -> PathBuf {
    home.join("bin").join(if os == HostOs::Windows {
        "java.exe"
    } else {
        "java"
    })
}

/// The `JAVA_HOME` entry to add to a `sdkmanager`/`avdmanager` [`Command`], if a JDK home is
/// recorded and still exists on disk. `None` means "use whatever `java` the environment gives".
pub async fn java_env(fs: &dyn Fs, data_dir: &Path, os: HostOs) -> Option<(String, String)> {
    let raw = fs.read(&marker_path(data_dir)).await.ok()?;
    let home = PathBuf::from(String::from_utf8_lossy(&raw).trim());
    if home.as_os_str().is_empty() || !fs.exists(&java_binary(&home, os)).await.unwrap_or(false) {
        return None;
    }
    Some(("JAVA_HOME".to_string(), home.display().to_string()))
}

/// Parse the major version out of `java -version`'s `"..."`-quoted version string. Handles both
/// the pre-JDK9 scheme (`"1.8.0_372"` = Java 8) and the JEP 223 scheme (`"17.0.9"`, `"21"`).
#[must_use]
pub fn parse_java_major_version(text: &str) -> Option<u32> {
    let start = text.find('"')? + 1;
    let rest = &text[start..];
    let end = rest.find('"')?;
    let ver = &rest[..end];
    let mut segments = ver.split(['.', '_', '-', '+']);
    let first: u32 = segments.next()?.parse().ok()?;
    if first == 1 {
        segments.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// `java.home` out of `java -XshowSettings:properties -version` (printed to stderr as
/// `    java.home = /path`; verified against a real JDK 21 on macOS, 2026-10-07).
fn parse_java_home(text: &str) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix("java.home")?
            .trim_start()
            .strip_prefix('=')?
            .trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

fn combined_output(out: &crate::ports::Output) -> String {
    format!("{}\n{}", out.stderr, out.stdout)
}

/// Ask `<home>/bin/java -version`; `Some(major)` when it's a working JDK 17+.
async fn probe_home(home: &Path, os: HostOs, ports: &BootstrapPorts<'_>) -> Option<u32> {
    let cmd = Command::new(java_binary(home, os).display().to_string()).arg("-version");
    let out = ports.process.run(cmd).await.ok()?;
    parse_java_major_version(&combined_output(&out)).filter(|m| *m >= MIN_JDK_MAJOR)
}

/// Ask `java` on `PATH`; also learns its home via `-XshowSettings:properties`.
async fn probe_path(ports: &BootstrapPorts<'_>) -> Option<(u32, Option<PathBuf>)> {
    let cmd = Command::new("java")
        .arg("-XshowSettings:properties")
        .arg("-version");
    let out = ports.process.run(cmd).await.ok()?;
    let text = combined_output(&out);
    let major = parse_java_major_version(&text).filter(|m| *m >= MIN_JDK_MAJOR)?;
    Some((major, parse_java_home(&text)))
}

/// Directories that may hold JDKs on this OS: `direct` are JDK homes themselves; each `parents`
/// entry is a folder whose children are installs, with `suffix` appended (macOS bundles keep
/// the home at `Contents/Home`).
#[derive(Debug, Default, PartialEq, Eq)]
struct Candidates {
    direct: Vec<PathBuf>,
    parents: Vec<(PathBuf, &'static str)>,
}

fn well_known(os: HostOs, env: &dyn Fn(&str) -> Option<String>) -> Candidates {
    let var = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut c = Candidates::default();
    match os {
        HostOs::MacOs => {
            c.direct.extend(
                [
                    "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
                    "/Applications/Android Studio Preview.app/Contents/jbr/Contents/Home",
                    "/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
                    "/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home",
                    "/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home",
                    "/usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
                    "/usr/local/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home",
                    "/usr/local/opt/openjdk/libexec/openjdk.jdk/Contents/Home",
                ]
                .map(PathBuf::from),
            );
            c.parents.push((
                PathBuf::from("/Library/Java/JavaVirtualMachines"),
                "Contents/Home",
            ));
            if let Some(home) = var("HOME") {
                c.parents.push((
                    home.join("Library/Java/JavaVirtualMachines"),
                    "Contents/Home",
                ));
                c.parents.push((home.join(".jdks"), "Contents/Home"));
                c.parents.push((home.join(".jdks"), ""));
                c.parents.push((home.join(".sdkman/candidates/java"), ""));
            }
        }
        HostOs::Linux => {
            c.direct.extend(
                [
                    "/opt/android-studio/jbr",
                    "/usr/local/android-studio/jbr",
                    "/snap/android-studio/current/android-studio/jbr",
                ]
                .map(PathBuf::from),
            );
            c.parents.push((PathBuf::from("/usr/lib/jvm"), ""));
            c.parents.push((PathBuf::from("/usr/java"), ""));
            if let Some(home) = var("HOME") {
                c.direct.push(home.join("android-studio/jbr"));
                c.parents.push((home.join(".jdks"), ""));
                c.parents.push((home.join(".sdkman/candidates/java"), ""));
            }
        }
        HostOs::Windows => {
            for key in ["ProgramFiles", "ProgramW6432"] {
                if let Some(pf) = var(key) {
                    c.direct.push(pf.join(r"Android\Android Studio\jbr"));
                    c.direct
                        .push(pf.join(r"Android\Android Studio Preview\jbr"));
                    for vendor in [
                        "Eclipse Adoptium",
                        "Microsoft",
                        "Java",
                        "Zulu",
                        "Amazon Corretto",
                        "BellSoft",
                        "Semeru",
                    ] {
                        c.parents.push((pf.join(vendor), ""));
                    }
                }
            }
            if let Some(local) = var("LOCALAPPDATA") {
                c.direct.push(local.join(r"Programs\Android Studio\jbr"));
                c.parents
                    .push((local.join(r"Programs\Eclipse Adoptium"), ""));
            }
            if let Some(user) = var("USERPROFILE") {
                c.parents.push((user.join(".jdks"), ""));
            }
        }
    }
    c
}

/// Every concrete JDK-home candidate, newest-looking name first within each parent.
async fn expand(c: Candidates, fs: &dyn Fs) -> Vec<PathBuf> {
    let mut out = c.direct;
    for (parent, suffix) in c.parents {
        let Ok(mut children) = fs.list_dir(&parent).await else {
            continue;
        };
        children.sort();
        children.reverse();
        for child in children {
            out.push(if suffix.is_empty() {
                child
            } else {
                child.join(suffix)
            });
        }
    }
    out
}

async fn remember(fs: &dyn Fs, data_dir: &Path, home: &Path) -> Result<()> {
    let marker = marker_path(data_dir);
    if let Some(dir) = marker.parent() {
        fs.ensure_dir(dir).await?;
    }
    fs.write_atomic(&marker, home.display().to_string().as_bytes())
        .await
}

/// Make sure a JDK 17+ is available and recorded, reusing an existing one wherever possible and
/// downloading Temurin 17 only as a last resort.
///
/// # Errors
/// [`CoreError::Unsupported`] when no JDK exists and none can be downloaded for this
/// OS/architecture; [`CoreError::Download`] / [`CoreError::Fs`] / [`CoreError::Process`] from the
/// download and unpack steps.
pub async fn ensure(
    data_dir: &Path,
    os: HostOs,
    arch: Option<HostArch>,
    env: &(dyn Fn(&str) -> Option<String> + Sync),
    ports: &BootstrapPorts<'_>,
    job: &JobHandle,
) -> Result<Jdk> {
    let found = |jdk: &Jdk| {
        let place = jdk
            .java_home
            .as_ref()
            .map_or_else(|| "PATH".to_string(), |h| h.display().to_string());
        job.report(Progress::log(format!(
            "using the Java {} already on this computer ({place})",
            jdk.major
        )));
    };

    // 1. What a previous run chose.
    if let Some((_, home)) = java_env(ports.fs, data_dir, os).await {
        let home = PathBuf::from(home);
        if let Some(major) = probe_home(&home, os, ports).await {
            let jdk = Jdk {
                java_home: Some(home),
                major,
                origin: JdkOrigin::Remembered,
            };
            found(&jdk);
            return Ok(jdk);
        }
    }

    // 2. `JAVA_HOME`.
    if let Some(home) = env("JAVA_HOME").filter(|h| !h.trim().is_empty()) {
        let home = PathBuf::from(home.trim());
        if let Some(major) = probe_home(&home, os, ports).await {
            remember(ports.fs, data_dir, &home).await?;
            let jdk = Jdk {
                java_home: Some(home),
                major,
                origin: JdkOrigin::JavaHomeEnv,
            };
            found(&jdk);
            return Ok(jdk);
        }
    }

    // 3. `java` on `PATH`.
    if let Some((major, home)) = probe_path(ports).await {
        if let Some(h) = &home {
            remember(ports.fs, data_dir, h).await?;
        }
        let jdk = Jdk {
            java_home: home,
            major,
            origin: JdkOrigin::Path,
        };
        found(&jdk);
        return Ok(jdk);
    }

    // 4. Well-known install locations.
    for home in expand(well_known(os, env), ports.fs).await {
        if let Some(major) = probe_home(&home, os, ports).await {
            remember(ports.fs, data_dir, &home).await?;
            let jdk = Jdk {
                java_home: Some(home),
                major,
                origin: JdkOrigin::WellKnown,
            };
            found(&jdk);
            return Ok(jdk);
        }
    }

    // 5. Nothing usable: download one.
    download_managed(data_dir, os, arch, ports, job).await
}

/// Where the Adoptium API says the latest Temurin `MANAGED_MAJOR` JDK archive lives.
#[derive(Debug, PartialEq, Eq)]
struct TemurinPackage {
    link: Url,
    sha256: String,
    name: String,
}

/// Parse `GET https://api.adoptium.net/v3/assets/latest/17/hotspot?...` — a JSON array whose
/// first element carries `binary.package.{link,checksum,name}` (shape captured from the live API
/// for mac/aarch64, windows/x64 and linux/x64 on 2026-10-07; see
/// `tests/fixtures/adoptium-latest-17-linux-x64.json`).
fn parse_temurin_package(json: &[u8]) -> Result<TemurinPackage> {
    let bad = |detail: &str| CoreError::Invalid {
        what: "Adoptium release list",
        detail: detail.to_string(),
    };
    let doc: serde_json::Value = serde_json::from_slice(json).map_err(|e| CoreError::Invalid {
        what: "Adoptium release list",
        detail: e.to_string(),
    })?;
    let pkg = doc
        .get(0)
        .and_then(|r| r.pointer("/binary/package"))
        .ok_or_else(|| bad("no Temurin build is published for this OS/architecture"))?;
    let text = |key: &str| {
        pkg.get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| bad(&format!("package.{key} is missing")))
    };
    Ok(TemurinPackage {
        link: text("link")?
            .parse()
            .map_err(|e: url::ParseError| bad(&e.to_string()))?,
        sha256: text("checksum")?.to_string(),
        name: text("name")?.to_string(),
    })
}

fn adoptium_os(os: HostOs) -> &'static str {
    match os {
        HostOs::Linux => "linux",
        HostOs::MacOs => "mac",
        HostOs::Windows => "windows",
    }
}

async fn download_managed(
    data_dir: &Path,
    os: HostOs,
    arch: Option<HostArch>,
    ports: &BootstrapPorts<'_>,
    job: &JobHandle,
) -> Result<Jdk> {
    let arch = arch.ok_or_else(|| {
        CoreError::Unsupported(
            "no Java 17+ found and this CPU architecture has no downloadable build; install a \
             JDK 17 or newer and retry"
                .to_string(),
        )
    })?;

    job.report(Progress {
        phase: Some("Setting up Java".to_string()),
        ..Progress::empty()
    });
    job.report(Progress::log(
        "no Java 17+ found on this computer — downloading one (Eclipse Temurin 17)",
    ));

    let jdk_dir = data_dir.join("jdk");
    let downloads = jdk_dir.join(".downloads");
    ports.fs.ensure_dir(&downloads).await?;

    let api: Url = format!(
        "https://api.adoptium.net/v3/assets/latest/{MANAGED_MAJOR}/hotspot?architecture={}&image_type=jdk&os={}&vendor=eclipse",
        arch.tag(),
        adoptium_os(os)
    )
    .parse()
    .map_err(|e: url::ParseError| CoreError::Invalid {
        what: "Adoptium API URL",
        detail: e.to_string(),
    })?;
    let listing = downloads.join("adoptium.json");
    ports.downloader.fetch(&api, &listing, None, job).await?;
    let package = parse_temurin_package(&ports.fs.read(&listing).await?)?;
    let _ = ports.fs.remove(&listing).await;

    let archive = downloads.join(&package.name);
    ports
        .downloader
        .fetch(&package.link, &archive, Some(&package.sha256), job)
        .await?;

    job.report(Progress::log("unpacking Java"));
    let dest = jdk_dir.join(format!("temurin-{MANAGED_MAJOR}"));
    let _ = ports.fs.remove(&dest).await;
    ports.fs.ensure_dir(&dest).await?;
    unpack(&archive, &dest, os, ports).await?;
    let _ = ports.fs.remove(&archive).await;

    // macOS bundles keep the home inside `Contents/Home`.
    let mac_home = dest.join("Contents/Home");
    let home = if ports.fs.exists(&java_binary(&mac_home, os)).await? {
        mac_home
    } else {
        dest
    };
    let major = probe_home(&home, os, ports).await.ok_or_else(|| {
        CoreError::Unsupported(format!(
            "downloaded Java to {} but it didn't start; install a JDK 17 or newer and retry",
            home.display()
        ))
    })?;
    remember(ports.fs, data_dir, &home).await?;
    job.report(Progress::log(format!("Java {major} ready")));
    Ok(Jdk {
        java_home: Some(home),
        major,
        origin: JdkOrigin::Downloaded,
    })
}

/// Windows ships a `.zip` (unpacked in-process, like `cmdline-tools`); macOS/Linux ship a
/// `.tar.gz`, unpacked with the system `tar` so symlinks and the executable bits survive.
async fn unpack(archive: &Path, dest: &Path, os: HostOs, ports: &BootstrapPorts<'_>) -> Result<()> {
    if os == HostOs::Windows {
        let bytes = ports.fs.read(archive).await?;
        let entries = tokio::task::spawn_blocking(move || read_zip_stripping_top_dir(&bytes))
            .await
            .map_err(|e| CoreError::Invalid {
                what: "Java archive",
                detail: format!("extraction task panicked: {e}"),
            })??;
        for entry in entries {
            let out = dest.join(&entry.relative);
            if entry.is_dir {
                ports.fs.ensure_dir(&out).await?;
                continue;
            }
            if let Some(parent) = out.parent() {
                ports.fs.ensure_dir(parent).await?;
            }
            ports.fs.write_atomic(&out, &entry.contents).await?;
        }
        return Ok(());
    }

    let out = ports
        .process
        .run(
            Command::new("tar")
                .arg("-xzf")
                .arg(archive.display().to_string())
                .arg("-C")
                .arg(dest.display().to_string())
                .arg("--strip-components=1"),
        )
        .await?;
    if out.success() {
        Ok(())
    } else {
        Err(CoreError::Process {
            program: "tar".to_string(),
            code: out.status,
            stderr: out.stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;
    use crate::model::job::JobId;
    use crate::ports::Output;
    use crate::testing::{FakeDownloader, FakeProcessRunner, InMemoryFs};

    const LINUX_FIXTURE: &[u8] =
        include_bytes!("../../tests/fixtures/adoptium-latest-17-linux-x64.json");

    fn out(stderr: &str) -> Output {
        Output {
            status: 0,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    fn jdk17() -> Output {
        out("openjdk version \"17.0.9\" 2023-10-17")
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn job() -> JobHandle {
        JobHandle::collector(JobId("j".into())).0
    }

    async fn run_ensure(
        fs: &InMemoryFs,
        dl: &FakeDownloader,
        process: &FakeProcessRunner,
        os: HostOs,
        env: &(dyn Fn(&str) -> Option<String> + Sync),
    ) -> Result<Jdk> {
        let ports = BootstrapPorts {
            fs,
            downloader: dl,
            process,
        };
        ensure(
            Path::new("/data"),
            os,
            Some(HostArch::X64),
            env,
            &ports,
            &job(),
        )
        .await
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
        assert_eq!(
            parse_java_major_version("openjdk version \"21-ea\" 2023-09-19"),
            Some(21)
        );
        assert_eq!(parse_java_major_version("not a version string"), None);
        // macOS's `/usr/bin/java` stub when no JDK is installed.
        assert_eq!(
            parse_java_major_version(
                "The operation couldn't be completed. Unable to locate a Java Runtime."
            ),
            None
        );
    }

    #[test]
    fn java_home_is_read_from_show_settings_output() {
        let text = "Property settings:\n    java.class.version = 65.0\n    java.home = /Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home\n    java.version = 21\n";
        assert_eq!(
            parse_java_home(text),
            Some(PathBuf::from(
                "/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home"
            ))
        );
        assert_eq!(parse_java_home("nothing here"), None);
    }

    #[test]
    fn well_known_locations_cover_android_studio_on_every_os() {
        let env = |k: &str| match k {
            "HOME" => Some("/home/me".to_string()),
            "ProgramFiles" => Some(r"C:\Program Files".to_string()),
            _ => None,
        };
        let mac = well_known(HostOs::MacOs, &env);
        assert!(mac
            .direct
            .iter()
            .any(|p| p.ends_with("Android Studio.app/Contents/jbr/Contents/Home")));
        let win = well_known(HostOs::Windows, &env);
        assert!(win
            .direct
            .iter()
            .any(|p| p.to_string_lossy().contains("Android Studio")));
        assert!(win
            .parents
            .iter()
            .any(|(p, _)| p.to_string_lossy().contains("Eclipse Adoptium")));
        let linux = well_known(HostOs::Linux, &env);
        assert!(linux
            .parents
            .iter()
            .any(|(p, _)| p == Path::new("/usr/lib/jvm")));
    }

    #[test]
    fn the_real_adoptium_response_parses() {
        let pkg = parse_temurin_package(LINUX_FIXTURE).expect("parses");
        assert!(pkg.name.ends_with(".tar.gz"));
        assert!(pkg
            .link
            .as_str()
            .starts_with("https://github.com/adoptium/"));
        assert_eq!(pkg.sha256.len(), 64);
    }

    #[test]
    fn an_empty_release_list_is_a_clear_error() {
        let err = parse_temurin_package(b"[]").unwrap_err();
        assert!(err.to_string().contains("no Temurin build"));
    }

    #[tokio::test]
    async fn reuses_java_on_path_and_remembers_its_home() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run(
            "java -XshowSettings:properties -version",
            out("    java.home = /opt/jdk17\nopenjdk version \"17.0.9\""),
        );
        let dl = FakeDownloader::new();
        let jdk = run_ensure(&fs, &dl, &process, HostOs::Linux, &no_env)
            .await
            .unwrap();
        assert_eq!(jdk.origin, JdkOrigin::Path);
        assert_eq!(jdk.java_home, Some(PathBuf::from("/opt/jdk17")));
        assert!(dl.fetched().is_empty());
        assert_eq!(
            fs.read(&marker_path(Path::new("/data"))).await.unwrap(),
            b"/opt/jdk17"
        );
    }

    #[tokio::test]
    async fn java_home_env_wins_over_path() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new()
            .on_run("/jdks/21/bin/java -version", out("openjdk version \"21\""));
        let env = |k: &str| (k == "JAVA_HOME").then(|| "/jdks/21".to_string());
        let jdk = run_ensure(&fs, &FakeDownloader::new(), &process, HostOs::Linux, &env)
            .await
            .unwrap();
        assert_eq!(jdk.origin, JdkOrigin::JavaHomeEnv);
        assert_eq!(jdk.major, 21);
    }

    #[tokio::test]
    async fn a_too_old_java_is_skipped_for_a_newer_well_known_one() {
        let fs = InMemoryFs::new();
        fs.ensure_dir(Path::new("/usr/lib/jvm/java-17-openjdk-amd64"))
            .await
            .unwrap();
        let process = FakeProcessRunner::new()
            .on_run(
                "java -XshowSettings:properties -version",
                out("openjdk version \"1.8.0_372\""),
            )
            .on_run("java-17-openjdk-amd64/bin/java -version", jdk17());
        let jdk = run_ensure(
            &fs,
            &FakeDownloader::new(),
            &process,
            HostOs::Linux,
            &no_env,
        )
        .await
        .unwrap();
        assert_eq!(jdk.origin, JdkOrigin::WellKnown);
        assert_eq!(
            jdk.java_home,
            Some(PathBuf::from("/usr/lib/jvm/java-17-openjdk-amd64"))
        );
    }

    #[tokio::test]
    async fn downloads_temurin_when_the_machine_has_no_java() {
        let archive = b"fake tar.gz bytes".to_vec();
        let sha = FakeDownloader::sha256_hex(&archive);
        let listing = format!(
            r#"[{{"binary":{{"package":{{"link":"https://example.test/jdk.tar.gz","checksum":"{sha}","name":"jdk.tar.gz"}}}}}}]"#
        );
        let fs = InMemoryFs::new();
        // What the real downloader would have written, and what `tar` would have unpacked.
        fs.write_atomic(
            Path::new("/data/jdk/.downloads/adoptium.json"),
            listing.as_bytes(),
        )
        .await
        .unwrap();
        fs.write_atomic(Path::new("/data/jdk/temurin-17/bin/java"), b"")
            .await
            .unwrap();
        let dl = FakeDownloader::new()
            .stub(
                "https://api.adoptium.net/v3/assets/latest/17/hotspot?architecture=x64&image_type=jdk&os=linux&vendor=eclipse",
                listing.clone().into_bytes(),
            )
            .stub("https://example.test/jdk.tar.gz", archive);
        let process = FakeProcessRunner::new()
            .on_run("tar -xzf /data/jdk/.downloads/jdk.tar.gz", out(""))
            .on_run("/data/jdk/temurin-17/bin/java -version", jdk17());

        let jdk = run_ensure(&fs, &dl, &process, HostOs::Linux, &no_env)
            .await
            .unwrap();
        assert_eq!(jdk.origin, JdkOrigin::Downloaded);
        assert_eq!(jdk.java_home, Some(PathBuf::from("/data/jdk/temurin-17")));
        assert_eq!(dl.fetched().len(), 2);
        // (`java_env` also checks the binary exists; the fake `tar` doesn't write files.)
        assert_eq!(
            fs.read(&marker_path(Path::new("/data"))).await.unwrap(),
            b"/data/jdk/temurin-17"
        );
    }

    #[tokio::test]
    async fn windows_zip_is_unpacked_in_process() {
        let mut zip_bytes = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("jdk-17.0.20+1/bin/java.exe", opts).unwrap();
            w.write_all(b"MZ").unwrap();
            w.finish().unwrap();
        }
        let sha = FakeDownloader::sha256_hex(&zip_bytes);
        let listing = format!(
            r#"[{{"binary":{{"package":{{"link":"https://example.test/jdk.zip","checksum":"{sha}","name":"jdk.zip"}}}}}}]"#
        );
        let fs = InMemoryFs::new();
        fs.write_atomic(
            Path::new("/data/jdk/.downloads/adoptium.json"),
            listing.as_bytes(),
        )
        .await
        .unwrap();
        fs.write_atomic(Path::new("/data/jdk/.downloads/jdk.zip"), &zip_bytes)
            .await
            .unwrap();
        let dl = FakeDownloader::new()
            .stub(
                "https://api.adoptium.net/v3/assets/latest/17/hotspot?architecture=x64&image_type=jdk&os=windows&vendor=eclipse",
                listing.into_bytes(),
            )
            .stub("https://example.test/jdk.zip", zip_bytes);
        let process = FakeProcessRunner::new()
            .on_run("java -XshowSettings:properties -version", out("not java"))
            .on_run("temurin-17/bin/java.exe -version", jdk17());

        let jdk = run_ensure(&fs, &dl, &process, HostOs::Windows, &no_env)
            .await
            .unwrap();
        assert_eq!(jdk.origin, JdkOrigin::Downloaded);
        assert!(fs
            .exists(Path::new("/data/jdk/temurin-17/bin/java.exe"))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn an_unreachable_download_surfaces_as_a_download_error() {
        let fs = InMemoryFs::new();
        let process = FakeProcessRunner::new().on_run("java -XshowSettings", out("nope"));
        let err = run_ensure(
            &fs,
            &FakeDownloader::new(),
            &process,
            HostOs::Linux,
            &no_env,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "download_failed");
    }

    #[tokio::test]
    async fn java_env_ignores_a_recorded_home_that_no_longer_exists() {
        let fs = InMemoryFs::new();
        fs.write_atomic(&marker_path(Path::new("/data")), b"/gone/jdk")
            .await
            .unwrap();
        assert_eq!(java_env(&fs, Path::new("/data"), HostOs::Linux).await, None);
    }
}
