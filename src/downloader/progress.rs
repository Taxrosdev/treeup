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
use treeup_core::downloader::{DownloadError, DownloadKind, Downloader};

#[derive(Clone)]
pub struct ProgressDownloader<D: Downloader> {
    downloader: Arc<D>,
    downloaded: Arc<AtomicU64>,
}

impl<D: Downloader> ProgressDownloader<D> {
    #[must_use]
    pub fn from_downloader(downloader: Arc<D>, downloaded: Arc<AtomicU64>) -> Self {
        Self {
            downloader,
            downloaded,
        }
    }
}

impl<D: Downloader> Downloader for ProgressDownloader<D> {
    async fn fetch(
        &self,
        hash: &[u8],
        kind: DownloadKind,
    ) -> Result<Pin<Box<impl Stream<Item = Result<Bytes, DownloadError>>>>, DownloadError> {
        let stream = self.downloader.fetch(hash, kind).await?;
        let downloaded = self.downloaded.clone();

        Ok(Box::pin(stream.map(move |r| {
            r.inspect(|chunk| {
                downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            })
        })))
    }

    fn remote(&self) -> String {
        self.downloader.remote()
    }
}
