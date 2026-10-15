use bytes::Bytes;
use reqwest::{StatusCode, header::RANGE};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use treeup_core::downloader::{DownloadError, Downloader, ObjectDownloader};

use crate::{downloader::ReqwestDownloader, object::cas::PackfileIndex};

pub struct PackfileDownloader {
    downloader: Arc<ReqwestDownloader>,
    index_cache: Mutex<Option<Arc<IndexCacheEntry>>>,
}

type IndexCacheEntry = HashMap<Vec<u8>, PackfileIndex>;

impl PackfileDownloader {
    #[must_use]
    pub fn from_downloader(downloader: Arc<ReqwestDownloader>) -> Self {
        let index_cache = Mutex::new(None);

        Self {
            downloader,
            index_cache,
        }
    }

    pub async fn get_index(&self) -> Result<Arc<IndexCacheEntry>, DownloadError> {
        let mut cache = self.index_cache.lock().await;
        if let Some(index) = &*cache {
            return Ok(index.clone());
        }

        let res = self
            .downloader
            .client
            .get(format!(
                "{}/{}",
                self.downloader.objects_base_url, "packfile.idx"
            ))
            .send()
            .await?;

        *cache = Some(Arc::new(PackfileIndex::parse_indexes(&res.bytes().await?)?));

        Ok(cache.clone().unwrap())
    }
}

impl Downloader for PackfileDownloader {
    fn remote(&self) -> String {
        self.downloader.remote()
    }
}

impl ObjectDownloader for PackfileDownloader {
    async fn fetch_object(&self, hash: &[u8]) -> Result<Bytes, DownloadError> {
        let index = self.get_index().await?;

        match index.get(hash) {
            None => Ok(self.downloader.fetch_object(hash).await?),
            Some(entry) => {
                let res = self
                    .downloader
                    .client
                    .get(format!("{}/packfile", self.downloader.objects_base_url))
                    .header(
                        RANGE,
                        format!("bytes={}-{}", entry.start, entry.start + entry.len - 1),
                    )
                    .send()
                    .await?
                    .error_for_status()?;

                if res.status() != StatusCode::PARTIAL_CONTENT {
                    return Err(Box::new(UnsupportedServer {}));
                }

                Ok(res.bytes().await?)
            }
        }
    }
}

#[derive(Debug)]
struct UnsupportedServer {}

impl std::fmt::Display for UnsupportedServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Server did not return partial content (206)")
    }
}

impl std::error::Error for UnsupportedServer {}
