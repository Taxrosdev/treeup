#[derive(snafu::Snafu, Debug)]
pub enum Error {
    #[snafu(display("Downloader failed: {source}"))]
    Downloader {
        // Automatically accepts an already-boxed error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    HashError {
        expected: String,
        received: String,
    },
    SerdeError {
        #[snafu(source)]
        source: serde_json::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
