use bytes::Bytes;
use futures_core::Stream;

pub type DownloadError = Box<dyn std::error::Error + Send + Sync>;

pub trait Downloader {
    fn remote(&self) -> String;
}

/// Utility to Fetch Blobs from remote `Repo`s.
pub trait BlobDownloader: Send + Sync + Downloader {
    fn fetch_blob(
        &self,
        hash: &[u8],
    ) -> impl Future<
        Output = Result<impl Stream<Item = Result<Bytes, DownloadError>> + Send, DownloadError>,
    > + Send;
}

/// Utility to Fetch Objects from remote `Repo`s.
pub trait ObjectDownloader: Send + Sync + Downloader {
    fn fetch_object(
        &self,
        hash: &[u8],
    ) -> impl Future<Output = Result<Bytes, DownloadError>> + Send;
}
