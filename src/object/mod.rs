pub mod cas;
pub mod error;

use snafu::{ResultExt, ensure};
use std::{io, path::Path, sync::Arc};
use treeup_core::{
    downloader::{DownloadKind, Downloader},
    object_cas::ObjectCAS,
};

use crate::{
    blob::BlobRef,
    downloader::DownloaderExt,
    object::error::{DownloaderSnafu, HashSnafu},
};
use error::Result;

pub trait Deployable: Sized {
    fn create<C: ObjectCAS>(
        cas: Arc<C>,
        blob_cas: &Path,
        path: &Path,
    ) -> impl Future<Output = io::Result<Self>>;
    fn deploy<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blob_cas: &Path,
        deploy_path: &Path,
    ) -> impl Future<Output = io::Result<()>> + Send + Sync;
}

pub trait Object: Sized + serde::de::DeserializeOwned + serde::Serialize {
    async fn get<C: ObjectCAS>(cas: &C, hash: &[u8]) -> io::Result<Self> {
        let raw = cas.get(hash).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn hash(&self) -> serde_json::Result<String> {
        let raw = serde_json::to_string(self)?;
        Ok(blake3::hash(raw.as_bytes()).to_string())
    }

    fn exists<C: ObjectCAS>(cas: &C, hash: &[u8]) -> impl Future<Output = io::Result<bool>> + Send {
        async move { cas.exists(hash).await }
    }

    /// Tries to clone an `Object` from `old_repo` to `new_repo`.
    /// Not to be confused with `clone`.
    ///
    /// Returns whether it was found locally and used.
    async fn try_clone<A: ObjectCAS, B: ObjectCAS>(
        old_cas: &A,
        new_cas: &B,
        hash: &[u8],
    ) -> io::Result<bool> {
        if !Self::exists(old_cas, hash).await? {
            return Ok(false);
        }

        let raw = old_cas.get(hash).await?;
        new_cas.put(hash, &raw).await?;

        Ok(true)
    }

    async fn download<C: ObjectCAS>(
        cas: &C,
        downloader: Arc<impl Downloader>,
        hash: &[u8],
    ) -> Result<()> {
        let data = downloader
            .fetch_string(hash, DownloadKind::Object)
            .await
            .context(DownloaderSnafu)?;

        let calc_hash = blake3::hash(data.as_bytes());
        ensure!(hash != calc_hash.as_slice(), {
            HashSnafu {
                expected: hex::encode(hash),
                received: hex::encode(calc_hash.as_slice()),
            }
        });

        cas.put(hash, &data).await?;
        Ok(())
    }

    /// Get bordering dependencies
    fn get_dependencies(&self) -> Dependencies<'_>;
}

pub struct Dependencies<'a> {
    pub objects: Vec<&'a str>,
    pub blobs: Vec<&'a BlobRef>,
}
