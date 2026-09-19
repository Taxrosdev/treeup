use bytes::Bytes;
use futures_core::Stream;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio_stream::StreamExt;
use treeup_core::downloader::{BlobDownloader, DownloadError, Downloader, ObjectDownloader};

#[derive(Clone)]
pub struct ProgressDownloader<D: Downloader> {
    downloader: Arc<D>,
    bytes_downloaded: Arc<AtomicU64>,
}

impl<D: Downloader> ProgressDownloader<D> {
    #[must_use]
    pub fn from_downloader(downloader: Arc<D>, bytes_downloaded: Arc<AtomicU64>) -> Self {
        Self {
            downloader,
            bytes_downloaded,
        }
    }
}

impl<D: Downloader> Downloader for ProgressDownloader<D> {
    fn remote(&self) -> String {
        self.downloader.remote()
    }
}

impl<D: BlobDownloader> BlobDownloader for ProgressDownloader<D> {
    async fn fetch_blob(
        &self,
        hash: &[u8],
    ) -> Result<impl Stream<Item = Result<Bytes, DownloadError>>, DownloadError> {
        let stream = self.downloader.fetch_blob(hash).await?;
        let bytes_downloaded = self.bytes_downloaded.clone();

        Ok(Box::pin(stream.map(move |r| {
            r.inspect(|chunk| {
                bytes_downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            })
        })))
    }
}

impl<D: ObjectDownloader> ObjectDownloader for ProgressDownloader<D> {
    async fn fetch_object(&self, hash: &[u8]) -> Result<Bytes, DownloadError> {
        let data = self.downloader.fetch_object(hash).await?;
        self.bytes_downloaded
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(data)
    }
}
