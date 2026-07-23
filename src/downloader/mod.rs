use async_trait::async_trait;

#[cfg(feature = "reqwest")]
mod reqwest;
#[cfg(feature = "reqwest")]
pub use reqwest::*;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn fetch(
        &self,
        hash: &str,
        kind: DownloadKind,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DownloadKind {
    Object,
    Blob,
}
