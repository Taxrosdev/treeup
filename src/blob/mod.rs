//! Blobs are Files stored on disks that are then hard-linked into their final location, this allows
//! for fast and quick IO and tree creation/deploying.

pub mod error;

use snafu::ResultExt;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    task,
};
use tokio_stream::StreamExt;
use treeup_core::downloader::{DownloadKind, Downloader};

use crate::{
    blob::error::Error,
    utils::{atomic::atomic_rename, permissions::Permissions},
};
use error::{DownloaderSnafu, HashDecodeSnafu, IoSnafu, Result};

/// A reference to a Blob, containing all information that may be required for deploying.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct BlobRef {
    hash: String,
    pub size: u64,

    #[serde(flatten)]
    #[serde(default)]
    permissions: Permissions,
}

impl BlobRef {
    /// Get the path on-disk of this Blob
    pub async fn local_path_with_parent(&self, blobs_path: &Path) -> io::Result<PathBuf> {
        let parent_path = blobs_path.join(&self.hash[..2]);
        fs::create_dir_all(&parent_path).await?;
        Ok(parent_path.join(&self.hash[2..]))
    }

    /// Get the path on-disk of this Blob
    ///
    /// Does not try to automatically create the parent directory.
    #[must_use]
    pub fn local_path(&self, blobs_path: &Path) -> PathBuf {
        let parent_path = blobs_path.join(&self.hash[..2]);
        parent_path.join(&self.hash[2..])
    }

    pub async fn exists(&self, blobs_path: &Path) -> io::Result<bool> {
        let path = self.local_path(blobs_path);

        fs::try_exists(&path).await
    }

    /// Download the referenced Blob onto disk
    pub async fn download(
        &self,
        blobs_path: &Path,
        downloader: Arc<impl Downloader>,
    ) -> Result<()> {
        let path = self
            .local_path_with_parent(blobs_path)
            .await
            .context(IoSnafu)?;
        let tmp_path = path.with_extension("tmp");
        let mut tmp_file = File::create(&tmp_path).await?;
        let hash_raw = hex::decode(&self.hash).context(HashDecodeSnafu {
            hash: self.hash.clone(),
        })?;

        let mut stream = downloader
            .fetch(&hash_raw, DownloadKind::Blob)
            .await
            .context(DownloaderSnafu)?;

        let mut hasher = blake3::Hasher::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context(DownloaderSnafu)?;
            hasher.write_all(&chunk)?;
            tmp_file.write_all(&chunk).await?;
        }

        drop(tmp_file);

        let calc_hash = hasher.finalize();
        if hash_raw != calc_hash.as_slice() {
            fs::remove_file(tmp_path).await?;
            return Err(Error::HashError {
                expected: self.hash.clone(),
                received: calc_hash.to_hex().to_string(),
            });
        }

        atomic_rename(tmp_path, path).await?;
        Ok(())
    }

    /// Tries to clone a Blob from `old_repo` to `new_repo`.
    /// Not to be confused with `clone`.
    ///
    /// Returns whether it was found locally and used.
    pub async fn try_clone(
        &self,
        old_blobs_path: &Path,
        new_blobs_path: &Path,
    ) -> io::Result<bool> {
        if !self.exists(old_blobs_path).await? {
            return Ok(false);
        }

        let old_path = self.local_path(old_blobs_path);
        let new_path = self.local_path_with_parent(new_blobs_path).await?;

        if fs::hard_link(&old_path, &new_path).await.is_err() {
            // Fallback to copying. Installers are commonly on removable media, and not on the same
            // partition.
            fs::copy(old_path, new_path).await?;
        }

        Ok(true)
    }

    pub async fn create<P: AsRef<Path>>(blobs_path: &Path, path: P) -> io::Result<Self> {
        let hash_path = path.as_ref().to_owned();
        let hash = task::spawn_blocking(|| {
            let mut hasher = blake3::Hasher::new();
            hasher.update_mmap_rayon(hash_path)?;
            Ok::<String, io::Error>(hasher.finalize().to_string())
        });

        let permissions = Permissions::get(&path).await?;
        let hash = hash.await.map_err(io::Error::other)??;

        let blob = BlobRef {
            hash: hash.clone(),
            size: fs::metadata(&path).await?.len(),

            permissions,
        };
        let blob_path = blob.local_path_with_parent(blobs_path).await?;

        if !fs::try_exists(&blob_path).await? {
            fs::hard_link(&path, blob_path).await?;
        }

        Ok(blob)
    }

    pub async fn deploy(&self, blobs_path: &Path, deploy_path: &Path) -> io::Result<()> {
        let path = self.local_path(blobs_path);
        fs::hard_link(path, deploy_path).await?;

        Permissions::deploy(
            deploy_path,
            self.permissions.mode,
            self.permissions.uid,
            self.permissions.gid,
        )
        .await?;

        Ok(())
    }
}
