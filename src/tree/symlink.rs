use async_trait::async_trait;
use std::{io, os::unix::fs::MetadataExt, path::Path};
use stringlike::StringLike;
use tokio::fs;

use crate::{object::Deployable, repo::Repo};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Symlink {
    pub name: StringLike,
    pub target: StringLike,

    uid: u32,
    gid: Option<u32>,
}

#[async_trait]
impl Deployable for Symlink {
    async fn create(_repo: &Repo, path: &Path) -> io::Result<Self> {
        let target = fs::read_link(path).await?.as_os_str().to_os_string().into();
        let metadata = fs::symlink_metadata(path).await?;

        // Permissions
        let uid = metadata.uid();
        let gid = if uid == metadata.gid() {
            None
        } else {
            Some(metadata.gid())
        };

        Ok(Symlink {
            name: path
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            target,

            uid,
            gid,
        })
    }

    async fn deploy(&self, _repo: &Repo, deploy_parent_path: &Path) -> io::Result<()> {
        let deploy_path = deploy_parent_path.join(&self.name);
        fs::symlink(self.target.to_path_buf(), &deploy_path).await?;

        // Permissions
        let gid = match self.gid {
            Some(gid) => gid,
            None => self.uid,
        };
        std::os::unix::fs::lchown(deploy_path, Some(self.uid), Some(gid))?;

        Ok(())
    }
}
