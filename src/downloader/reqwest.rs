use async_trait::async_trait;

use super::{DownloadKind, Downloader};

pub struct ReqwestDownloader {
    client: reqwest::Client,
    objects_base_url: String,
    blobs_base_url: String,
}

#[async_trait]
impl Downloader for ReqwestDownloader {
    async fn fetch(
        &self,
        hash: &str,
        kind: DownloadKind,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let base_url = match kind {
            DownloadKind::Object => &self.objects_base_url,
            DownloadKind::Blob => &self.blobs_base_url,
        };

        let res = self
            .client
            .get(format!("{}/{}/{}", base_url, &hash[..2], &hash[2..]))
            .send()
            .await?;

        let res = res.error_for_status()?;

        Ok(res.bytes().await?.to_vec())
    }
}

impl ReqwestDownloader {
    #[must_use]
    pub fn new(objects_base_url: &str, blobs_base_url: &str) -> Self {
        let objects_base_url = objects_base_url.trim_end_matches('/');
        let blobs_base_url = blobs_base_url.trim_end_matches('/');

        Self {
            client: reqwest::Client::new(),
            objects_base_url: objects_base_url.to_string(),
            blobs_base_url: blobs_base_url.to_string(),
        }
    }
}
