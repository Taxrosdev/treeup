use std::path::PathBuf;

use crate::downloader::Downloader;

pub struct Repo {
    pub objects_path: PathBuf,
    pub blobs_path: PathBuf,
    pub downloader: Box<dyn Downloader>,
}
