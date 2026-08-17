use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;

pub type DownloadError = Box<dyn std::error::Error + Send + Sync>;

/// Utiltity to Fetch from a remote `Repo`
pub trait Downloader: Send + Sync {
    fn fetch(
        &self,
        hash: &[u8],
        kind: DownloadKind,
    ) -> impl Future<
        Output = Result<
            Pin<Box<impl Stream<Item = Result<Bytes, DownloadError>> + Send>>,
            DownloadError,
        >,
    > + Send;

    fn remote(&self) -> String;
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DownloadKind {
    Object,
    Blob,
}
