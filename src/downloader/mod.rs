mod reqwest;
pub use reqwest::*;
mod progress;
pub use progress::*;
mod packfile;
pub use packfile::*;

use treeup_core::downloader::{DownloadError, ObjectDownloader};

pub trait DownloaderExt {
    fn fetch_string(
        &self,
        hash: &[u8],
    ) -> impl Future<Output = Result<String, DownloadError>> + Send;
}

impl<D: ObjectDownloader> DownloaderExt for D {
    async fn fetch_string(&self, hash: &[u8]) -> Result<String, DownloadError> {
        let data = self.fetch_object(hash).await?;

        Ok(String::from_utf8(data.to_vec())?)
    }
}
