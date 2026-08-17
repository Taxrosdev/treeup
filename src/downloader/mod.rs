mod reqwest;
pub use reqwest::*;
mod progress;
pub use progress::*;

use tokio_stream::StreamExt;
use treeup_core::downloader::{DownloadError, DownloadKind, Downloader};

pub trait DownloaderExt {
    fn fetch_string(
        &self,
        hash: &[u8],
        kind: DownloadKind,
    ) -> impl Future<Output = Result<String, DownloadError>> + Send;
}

impl<D: Downloader> DownloaderExt for D {
    async fn fetch_string(&self, hash: &[u8], kind: DownloadKind) -> Result<String, DownloadError> {
        let mut fetch = self.fetch(hash, kind).await?;
        let mut data = Vec::new();

        while let Some(chunk) = fetch.next().await {
            data.extend_from_slice(&chunk?);
        }

        Ok(String::from_utf8(data)?)
    }
}
