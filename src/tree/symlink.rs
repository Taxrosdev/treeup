use std::{io, path::Path};
use tokio::fs;

use crate::utils::permissions::Permissions;
use crate::utils::stringlike::StringLike;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Symlink {
    pub name: StringLike,
    pub target: StringLike,

    #[serde(flatten)]
    #[serde(default)]
    permissions: Permissions,
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

            permissions: Permissions {
                mode: None,
                uid: permissions.uid,
                gid: permissions.gid,
            },
        })
    }

    pub async fn deploy(&self, deploy_parent_path: &Path) -> io::Result<()> {
        let deploy_path = deploy_parent_path.join(&self.name);
        fs::symlink(self.target.to_path_buf(), &deploy_path).await?;

        Permissions::deploy(
            &deploy_path,
            self.permissions.mode,
            self.permissions.uid,
            self.permissions.gid,
        )
        .await?;

        Ok(())
    }
}
