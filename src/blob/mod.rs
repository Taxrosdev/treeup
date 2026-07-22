use async_trait::async_trait;
use std::{
    fs::Permissions,
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tokio::fs;

use crate::{downloader::DownloadKind, object::Deployable, repo::Repo};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct BlobRef {
    hash: String,

    mode: u32,
    uid: u32,
    gid: Option<u32>,
}

impl BlobRef {
    pub async fn local_path(repo: &Repo, hash: &str) -> io::Result<PathBuf> {
        let parent_path = repo.blobs_path.join(&hash[..2]);
        fs::create_dir_all(&parent_path).await?;
        Ok(parent_path.join(&hash[2..]))
    }

    pub async fn download(&self, repo: &Repo) -> crate::error::Result<()> {
        let path = Self::local_path(repo, &self.hash).await?;

        if fs::try_exists(&path).await? {
            return Ok(());
        }

        let raw = repo
            .downloader
            .fetch(&self.hash, DownloadKind::Blob)
            .await
            .map_err(crate::error::Error::DownloaderError)?;

        let calc_hash = blake3::hash(&raw).to_hex().to_string();
        if self.hash != calc_hash {
            return Err(crate::Error::HashError(self.hash.clone(), calc_hash));
        }

        fs::write(path, raw).await?;
        Ok(())
    }
}

#[async_trait]
impl Deployable for BlobRef {
    async fn create(repo: &Repo, path: &Path) -> io::Result<Self> {
        let mut hasher = blake3::Hasher::new();
        hasher.update_mmap_rayon(path)?;
        let hash = hasher.finalize().to_string();

        let blob_path = Self::local_path(repo, &hash).await?;

        fs::hard_link(path, blob_path).await?;

        // Permissions
        let metadata = fs::metadata(path).await?;
        let uid = metadata.uid();
        let gid = if uid == metadata.gid() {
            None
        } else {
            Some(metadata.gid())
        };

        Ok(BlobRef {
            hash: hash.clone(),
            uid,
            gid,
            mode: metadata.mode(),
        })
    }

    async fn deploy(&self, repo: &Repo, deploy_path: &Path) -> io::Result<()> {
        let path = Self::local_path(repo, &self.hash).await?;
        fs::hard_link(path, deploy_path).await?;

        // Permissions
        let gid = match self.gid {
            Some(gid) => gid,
            None => self.uid,
        };
        std::os::unix::fs::chown(deploy_path, Some(self.uid), Some(gid))?;
        let permissions = Permissions::from_mode(self.mode);
        fs::set_permissions(deploy_path, permissions).await?;

        Ok(())
    }
}
