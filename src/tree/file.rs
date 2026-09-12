use std::{io, path::Path};

use crate::{blob::BlobRef, utils::stringlike::StringLike};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct File {
    pub name: StringLike,
    pub blob: BlobRef,
}

impl File {
    pub async fn create<P: AsRef<Path>>(blobs_path: &Path, path: P) -> io::Result<Self> {
        Ok(File {
            name: path
                .as_ref()
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            blob: BlobRef::create(blobs_path, path).await?,
        })
    }

    pub async fn deploy<P: AsRef<Path>>(
        &self,
        blobs_path: &Path,
        deploy_parent_path: P,
    ) -> io::Result<()> {
        let deploy_path = deploy_parent_path.as_ref().join(self.name.to_os_string());

        self.blob.deploy(blobs_path, &deploy_path).await?;

        Ok(())
    }
}
