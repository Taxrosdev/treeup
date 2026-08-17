use std::sync::Arc;
use std::{io, path::Path};
use tokio::fs;
use treeup_core::object_cas::ObjectCAS;

use crate::object::Deployable;
use crate::utils::permissions::Permissions;
use crate::utils::stringlike::StringLike;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Symlink {
    pub name: StringLike,
    pub target: StringLike,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    uid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    gid: Option<u32>,
}

impl Deployable for Symlink {
    async fn create<C: ObjectCAS>(
        _cas: Arc<C>,
        _blobs_path: &Path,
        path: &Path,
    ) -> io::Result<Self> {
        let target = fs::read_link(path).await?.as_os_str().to_os_string().into();
        let permissions = Permissions::get(path).await?;

        Ok(Symlink {
            name: path
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            target,

            uid: permissions.uid,
            gid: permissions.gid,
        })
    }

    async fn deploy<C: ObjectCAS>(
        &self,
        _cas: Arc<C>,
        _blobs_path: &Path,
        deploy_parent_path: &Path,
    ) -> io::Result<()> {
        let deploy_path = deploy_parent_path.join(&self.name);
        fs::symlink(self.target.to_path_buf(), &deploy_path).await?;

        Permissions::deploy(deploy_path.to_path_buf(), None, self.uid, self.gid).await?;

        Ok(())
    }
}
