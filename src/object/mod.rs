use async_trait::async_trait;
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::fs;

use crate::{blob::BlobRef, repo::Repo};

#[async_trait]
pub trait Deployable: Sized {
    async fn create(repo: &Repo, path: &Path) -> io::Result<Self>;
    async fn deploy(&self, repo: &Repo, deploy_path: &Path) -> io::Result<()>;
}

#[async_trait]
pub trait Object: Sized + serde::de::DeserializeOwned + serde::Serialize {
    async fn local_path(repo: &Repo, hash: &str) -> io::Result<PathBuf> {
        let parent_path = repo.objects_path.join(&hash[..2]);
        fs::create_dir_all(&parent_path).await?;
        Ok(parent_path.join(&hash[2..]))
    }

    async fn get(repo: &Repo, hash: &str) -> io::Result<Self> {
        let path = Self::local_path(repo, hash).await?;
        let raw = fs::read(path).await?;
        Ok(serde_json::from_slice(&raw)?)
    }

    async fn download(repo: &Repo, hash: &str) -> crate::error::Result<Self> {
        let path = Self::local_path(repo, hash).await?;

        if fs::try_exists(&path).await? {
            let object = Self::get(repo, hash).await?;
            return Ok(object);
        }

        let raw = repo
            .downloader
            .fetch(hash)
            .await
            .map_err(crate::error::Error::DownloaderError)?;

        let calc_hash = blake3::hash(&raw).to_hex().to_string();
        if hash != calc_hash {
            return Err(crate::Error::HashError(hash.to_string(), calc_hash));
        }

        fs::write(path, &raw).await?;
        let object = serde_json::from_slice(&raw)?;
        Ok(object)
    }

    /// Get bordering dependencies
    fn get_dependencies(&self) -> Dependencies;
}

pub struct Dependencies<'a> {
    pub objects: Vec<&'a str>,
    pub blobs: Vec<&'a BlobRef>,
}
