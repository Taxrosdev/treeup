use async_trait::async_trait;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn fetch(&self, hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

pub struct ReqwestDownloader {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl Downloader for ReqwestDownloader {
    async fn fetch(&self, hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let res = self
            .client
            .get(format!(
                "{}/objects/{}/{}",
                self.base_url.clone(),
                &hash[..2],
                &hash[2..]
            ))
            .send()
            .await?;

        let res = res.error_for_status()?;

        Ok(res.bytes().await?.to_vec())
    }
}

impl ReqwestDownloader {
    fn new(base_url: &str) -> Self {
        let base_url = base_url.trim_end_matches('/');

        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }
}
