use std::{io, path::Path, sync::Arc};

use treeup_core::object_cas::ObjectCAS;

use crate::{blob::BlobRef, object::Deployable, utils::stringlike::StringLike};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct File {
    pub name: StringLike,
    pub blob: BlobRef,
}

impl Deployable for File {
    async fn create<C: ObjectCAS>(cas: Arc<C>, blobs_path: &Path, path: &Path) -> io::Result<Self> {
        Ok(File {
            name: path
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            blob: BlobRef::create(cas, blobs_path, path).await?,
        })
    }

    async fn deploy<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blobs_path: &Path,
        deploy_parent_path: &Path,
    ) -> io::Result<()> {
        let deploy_path = deploy_parent_path.join(self.name.to_os_string());

        self.blob.deploy(cas, blobs_path, &deploy_path).await?;

        Ok(())
    }
}
