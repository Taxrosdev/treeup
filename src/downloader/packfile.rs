use bytes::Bytes;
use reqwest::{StatusCode, header::RANGE};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use treeup_core::downloader::{DownloadError, Downloader, ObjectDownloader};

use crate::{downloader::ReqwestDownloader, object::cas::PackfileIndex};

pub struct PackfileDownloader {
    downloader: Arc<ReqwestDownloader>,
    index_cache: HashMap<u8, Mutex<Option<Arc<IndexCacheEntry>>>>,
}

type IndexCacheEntry = HashMap<Vec<u8>, PackfileIndex>;

impl PackfileDownloader {
    #[must_use]
    pub fn from_downloader(downloader: Arc<ReqwestDownloader>) -> Self {
        let mut index_cache = HashMap::new();
        for i in 0..=u8::MAX {
            index_cache.insert(i, Mutex::new(None));
        }

        Self {
            downloader,
            index_cache,
        }
    }

    pub async fn get_index(&self, hash_prefix: u8) -> Result<Arc<IndexCacheEntry>, DownloadError> {
        let mut cache = self.index_cache.get(&hash_prefix).unwrap().lock().await;
        if let Some(index) = &*cache {
            return Ok(index.clone());
        };

        let hash_str = hex::encode([hash_prefix]);
        let res = self
            .downloader
            .client
            .get(format!(
                "{}/{}/{}",
                self.downloader.objects_base_url,
                &hash_str[..2],
                "packfile.idx"
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
        let hash_str = hex::encode(hash);
        let index = self.get_index(hash[0]).await?;

        match index.get(&hash[1..]) {
            None => Ok(self.downloader.fetch_object(hash).await?),
            Some(entry) => {
                let res = self
                    .downloader
                    .client
                    .get(format!(
                        "{}/{}/packfile",
                        self.downloader.objects_base_url,
                        &hash_str[..2]
                    ))
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
