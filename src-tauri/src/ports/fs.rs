//! Real [`Fs`] backed by `tokio::fs`.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use emu_core::ports::Fs;
use emu_core::Result;
use tokio::io::AsyncWriteExt;

use super::{fs_err, tmp_sibling};

/// The real filesystem.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeFs;

#[async_trait]
impl Fs for NativeFs {
    async fn ensure_dir(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| fs_err(path, &e))
    }

    async fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let tmp = tmp_sibling(path);
        {
            let mut file = tokio::fs::File::create(&tmp)
                .await
                .map_err(|e| fs_err(&tmp, &e))?;
            file.write_all(bytes).await.map_err(|e| fs_err(&tmp, &e))?;
            file.flush().await.map_err(|e| fs_err(&tmp, &e))?;
        }
        tokio::fs::rename(&tmp, path)
            .await
            .map_err(|e| fs_err(path, &e))
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path).await.map_err(|e| fs_err(path, &e))
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut reader = tokio::fs::read_dir(path)
            .await
            .map_err(|e| fs_err(path, &e))?;
        let mut out = Vec::new();
        while let Some(entry) = reader.next_entry().await.map_err(|e| fs_err(path, &e))? {
            out.push(entry.path());
        }
        Ok(out)
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        tokio::fs::try_exists(path)
            .await
            .map_err(|e| fs_err(path, &e))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ensure_dir_write_read_list_remove_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fs = NativeFs;

        let nested = dir.path().join("a/b/c");
        fs.ensure_dir(&nested).await.expect("ensure_dir");
        assert!(fs.exists(&nested).await.expect("exists"));

        let file = nested.join("data.bin");
        fs.write_atomic(&file, b"hello").await.expect("write");
        assert_eq!(fs.read(&file).await.expect("read"), b"hello");

        // write_atomic never leaves the temp file behind.
        let leftovers: Vec<_> = fs
            .list_dir(&nested)
            .await
            .expect("list_dir")
            .into_iter()
            .filter(|p| p != &file)
            .collect();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");

        let listed = fs.list_dir(&nested).await.expect("list_dir");
        assert_eq!(listed, vec![file.clone()]);

        fs.remove(&file).await.expect("remove file");
        assert!(!fs.exists(&file).await.expect("exists after remove"));

        fs.remove(&dir.path().join("a")).await.expect("remove tree");
        assert!(!fs.exists(&nested).await.expect("exists after tree remove"));
    }

    #[tokio::test]
    async fn write_atomic_overwrites_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fs = NativeFs;
        let file = dir.path().join("f.txt");
        fs.write_atomic(&file, b"first").await.expect("write 1");
        fs.write_atomic(&file, b"second").await.expect("write 2");
        assert_eq!(fs.read(&file).await.expect("read"), b"second");
    }

    #[tokio::test]
    async fn missing_path_is_a_core_error_not_a_panic() {
        let fs = NativeFs;
        let err = fs.read(Path::new("/does/not/exist/at/all")).await;
        assert_eq!(err.unwrap_err().code(), "fs_error");
    }
}
