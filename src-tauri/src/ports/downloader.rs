//! Real [`Downloader`] backed by `reqwest`, streamed to disk with SHA-256 verification.
//!
//! Note (task 0010/0011): Google's Android SDK repository manifest only publishes **SHA-1**
//! checksums (`crates/emu-android/src/catalog.rs`), while this trait verifies **SHA-256**
//! (`expected_sha256`). That mismatch is real and unresolved here — this port only implements
//! what the trait already promises; task 0012 (which is the first caller that has a
//! `Component.sha1` in hand) decides how the two reconcile, e.g. verifying SHA-1 itself before
//! calling `fetch` with `expected_sha256: None`. Don't let that decision get made by accident
//! inside this file.

use std::path::Path;

use async_trait::async_trait;
use emu_core::error::CoreError;
use emu_core::model::job::JobHandle;
use emu_core::ports::{pct_progress, Downloader, Verified};
use emu_core::Result;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use url::Url;

use super::{fs_err, tmp_sibling};

/// Fetches over HTTP(S) via a shared `reqwest::Client`.
#[derive(Debug, Clone)]
pub struct NativeDownloader {
    client: reqwest::Client,
}

impl NativeDownloader {
    /// Build a downloader with a fresh HTTP client.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for NativeDownloader {
    fn default() -> Self {
        Self::new()
    }
}

fn download_err(url: &Url, detail: impl std::fmt::Display) -> CoreError {
    CoreError::Download {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}

/// Lower-case hex of a SHA-256 digest — same format `emu_core::testing::FakeDownloader` uses.
fn hex(digest: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[async_trait]
impl Downloader for NativeDownloader {
    async fn fetch(
        &self,
        url: &Url,
        into: &Path,
        expected_sha256: Option<&str>,
        job: &JobHandle,
    ) -> Result<Verified> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| download_err(url, e))?
            .error_for_status()
            .map_err(|e| download_err(url, e))?;
        let total = response.content_length();

        let tmp = tmp_sibling(into);
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| fs_err(&tmp, &e))?;

        let mut hasher = Sha256::new();
        let mut done: u64 = 0;
        let mut last_pct = None;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| download_err(url, e))?;
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| fs_err(&tmp, &e))?;
            done += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
            if let Some(total) = total {
                let progress = pct_progress(done, total);
                if progress.pct != last_pct {
                    last_pct = progress.pct;
                    job.report(progress);
                }
            }
        }
        file.flush().await.map_err(|e| fs_err(&tmp, &e))?;
        drop(file);

        let sha256 = hex(&hasher.finalize());
        if let Some(expected) = expected_sha256 {
            if !expected.eq_ignore_ascii_case(&sha256) {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(download_err(
                    url,
                    format!("sha256 mismatch: expected {expected}, got {sha256}"),
                ));
            }
        }

        if let Some(parent) = into.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| fs_err(parent, &e))?;
        }
        tokio::fs::rename(&tmp, into)
            .await
            .map_err(|e| fs_err(into, &e))?;

        Ok(Verified {
            path: into.to_path_buf(),
            sha256,
            bytes: done,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emu_core::model::job::JobId;
    use tokio::io::{AsyncReadExt, AsyncWriteExt as _};
    use tokio::net::TcpListener;

    /// A one-shot HTTP/1.0 server: accepts a single connection, ignores the request, and writes
    /// `body` back with a `Content-Length` header. Just enough to exercise a real network fetch
    /// without a real internet call in `just validate`.
    async fn serve_once(body: &'static [u8]) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await; // drain the request, ignore it
            let header = format!(
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(body).await;
            let _ = socket.shutdown().await;
        });
        Url::parse(&format!("http://{addr}/file.bin")).expect("url")
    }

    #[tokio::test]
    async fn fetch_streams_body_and_reports_verified_sha256() {
        let body = b"the quick brown fox jumps over the lazy dog";
        let url = serve_once(body).await;
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("out.bin");

        let expected_sha256 = hex(&Sha256::digest(body));
        let job = JobHandle::noop(JobId("dl".into()));
        let got = NativeDownloader::new()
            .fetch(&url, &dest, Some(&expected_sha256), &job)
            .await
            .expect("fetch");

        assert_eq!(got.path, dest);
        assert_eq!(got.sha256, expected_sha256);
        assert_eq!(got.bytes, body.len() as u64);
        assert_eq!(tokio::fs::read(&dest).await.expect("read dest"), body);
        // No leftover temp file next to the destination.
        let siblings: Vec<_> = tokio::fs::read_dir(dir.path())
            .await
            .expect("read_dir")
            .next_entry()
            .await
            .expect("next_entry")
            .into_iter()
            .map(|e| e.path())
            .collect();
        assert_eq!(siblings, vec![dest]);
    }

    #[tokio::test]
    async fn fetch_rejects_checksum_mismatch_and_cleans_up() {
        let body = b"payload";
        let url = serve_once(body).await;
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("out.bin");

        let job = JobHandle::noop(JobId("dl".into()));
        let err = NativeDownloader::new()
            .fetch(&url, &dest, Some("0".repeat(64).as_str()), &job)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "download_failed");
        assert!(!dest.exists());
        let leftover = tokio::fs::read_dir(dir.path())
            .await
            .expect("read_dir")
            .next_entry()
            .await
            .expect("next_entry");
        assert!(
            leftover.is_none(),
            "checksum failure must not leave a temp file behind, found {leftover:?}"
        );
    }
}
