use bytes::Bytes;
use futures_core::Stream;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio_stream::StreamExt;

use crate::downloader::{DownloadError, DownloadKind, Downloader, ReqwestDownloader};

#[derive(Clone)]
pub struct ProgressDownloader {
    reqwest: Arc<ReqwestDownloader>,
    downloaded: Arc<AtomicU64>,
}

impl ProgressDownloader {
    #[must_use]
    pub fn from_reqwest_downloader(
        reqwest: Arc<ReqwestDownloader>,
        downloaded: Arc<AtomicU64>,
    ) -> Self {
        Self {
            reqwest,
            downloaded,
        }
    }
}

impl Downloader for ProgressDownloader {
    async fn fetch(
        &self,
        hash: &str,
        kind: DownloadKind,
    ) -> Result<Pin<Box<impl Stream<Item = Result<Bytes, DownloadError>>>>, DownloadError> {
        let stream = self.reqwest.fetch(hash, kind).await?;
        let downloaded = self.downloaded.clone();

        Ok(Box::pin(stream.map(move |r| {
            r.inspect(|chunk| {
                downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            })
        })))
    }

    fn remote(&self) -> String {
        self.reqwest.remote()
    }
}
