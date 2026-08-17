use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;
use tokio_stream::StreamExt;
use treeup_core::downloader::{DownloadError, DownloadKind, Downloader};

/// Works for default `BasicFS`-like `ObjectCAS` Implementations
#[derive(Clone)]
pub struct ReqwestDownloader {
    pub(crate) client: reqwest::Client,
    pub(crate) objects_base_url: String,
    pub(crate) blobs_base_url: String,
    pub(crate) remote: String,
}

impl Downloader for ReqwestDownloader {
    async fn fetch(
        &self,
        hash: &[u8],
        kind: DownloadKind,
    ) -> Result<Pin<Box<impl Stream<Item = Result<Bytes, DownloadError>> + Send>>, DownloadError>
    {
        let hash_str = hex::encode(hash);
        let base_url = match kind {
            DownloadKind::Object => &self.objects_base_url,
            DownloadKind::Blob => &self.blobs_base_url,
        };

        let res = self
            .client
            .get(format!(
                "{}/{}/{}",
                base_url,
                &hash_str[..2],
                &hash_str[2..]
            ))
            .send()
            .await?;

        let res = res.error_for_status()?;

        Ok(Box::pin(res.bytes_stream().map(|r| {
            r.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })))
    }

    fn remote(&self) -> String {
        self.remote.clone()
    }
}

impl ReqwestDownloader {
    #[must_use]
    pub fn new(objects_base_url: &str, blobs_base_url: &str, remote: String) -> Self {
        let objects_base_url = objects_base_url.trim_end_matches('/');
        let blobs_base_url = blobs_base_url.trim_end_matches('/');

        Self {
            client: reqwest::Client::new(),
            objects_base_url: objects_base_url.to_string(),
            blobs_base_url: blobs_base_url.to_string(),
            remote: remote.into(),
        }
    }
}
