use async_trait::async_trait;
use std::{io, path::Path};
use stringlike::StringLike;
use tokio::fs;

#[cfg(all(feature = "ownership", unix))]
use crate::utils::permissions::Permissions;
use crate::{object::Deployable, repo::Repo};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Symlink {
    pub name: StringLike,
    pub target: StringLike,

    #[cfg(all(feature = "ownership", unix))]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    uid: Option<u32>,
    #[cfg(all(feature = "ownership", unix))]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    gid: Option<u32>,
}

#[async_trait]
impl Deployable for Symlink {
    async fn create(_repo: &Repo, path: &Path) -> io::Result<Self> {
        let target = fs::read_link(path).await?.as_os_str().to_os_string().into();

        #[cfg(all(feature = "ownership", unix))]
        let permissions = Permissions::get(path).await?;

        Ok(Symlink {
            name: path
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            target,

            #[cfg(all(feature = "ownership", unix))]
            uid: permissions.uid,
            #[cfg(all(feature = "ownership", unix))]
            gid: permissions.gid,
        })
    }

    async fn deploy(&self, _repo: &Repo, deploy_parent_path: &Path) -> io::Result<()> {
        let deploy_path = deploy_parent_path.join(&self.name);
        fs::symlink(self.target.to_path_buf(), &deploy_path).await?;

        #[cfg(all(feature = "ownership", unix))]
        Permissions::deploy_ownership(deploy_path, self.uid, self.gid).await?;

        Ok(())
    }
}
