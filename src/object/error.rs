use crate::downloader::DownloadError;

#[derive(snafu::Snafu, Debug)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Downloader failed: {source}"))]
    Downloader { source: DownloadError },
    #[snafu(display("Invalid hash, expected: {expected}, received: {received}"))]
    HashError { expected: String, received: String },
    IoError {
        #[snafu(source)]
        source: std::io::Error,
    },
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError { source: value }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
