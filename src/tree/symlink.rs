use std::{io, path::Path};
use tokio::fs;

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

impl Symlink {
    pub async fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let target = fs::read_link(&path)
            .await?
            .as_os_str()
            .to_os_string()
            .into();
        let permissions = Permissions::get(&path).await?;

        Ok(Symlink {
            name: path
                .as_ref()
                .file_name()
                .ok_or(io::ErrorKind::InvalidFilename)?
                .to_os_string()
                .into(),
            target,

            uid: permissions.uid,
            gid: permissions.gid,
        })
    }

    pub async fn deploy<P: AsRef<Path>>(&self, deploy_parent_path: P) -> io::Result<()> {
        let deploy_path = deploy_parent_path.as_ref().join(&self.name);
        fs::symlink(self.target.to_path_buf(), &deploy_path).await?;

        Permissions::deploy(deploy_path, None, self.uid, self.gid).await?;

        Ok(())
    }
}
