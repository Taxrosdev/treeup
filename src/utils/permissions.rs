use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{io, path::Path};
use tokio::fs;

#[cfg(all(any(feature = "mode", feature = "ownership"), unix))]
pub struct Permissions {
    #[cfg(feature = "mode")]
    pub mode: Option<u32>,
    #[cfg(feature = "ownership")]
    pub uid: Option<u32>,
    #[cfg(feature = "ownership")]
    pub gid: Option<u32>,
}

impl Permissions {
    pub async fn get(path: impl AsRef<Path>) -> io::Result<Permissions> {
        let metadata = fs::metadata(path).await?;

        Ok(Permissions {
            #[cfg(feature = "mode")]
            mode: Some(metadata.mode()),
            #[cfg(feature = "ownership")]
            uid: Some(metadata.uid()),
            #[cfg(feature = "ownership")]
            gid: Some(metadata.gid()),
        })
    }

    #[cfg(all(feature = "mode", unix))]
    pub async fn deploy_mode(path: impl AsRef<Path>, mode: Option<u32>) -> io::Result<()> {
        cfg_select! {
            all(feature="mode", unix) => {
                if let Some(mode) = mode {
                    let permissions = std::fs::Permissions::from_mode(mode);
                    fs::set_permissions(&path, permissions).await?;
                };
            },
            _ => ()
        };

        Ok(())
    }

    #[cfg(all(feature = "ownership", unix))]
    pub async fn deploy_ownership(
        path: impl AsRef<Path>,
        uid: Option<u32>,
        gid: Option<u32>,
    ) -> io::Result<()> {
        use std::os::unix::fs::lchown;

        cfg_select! {
            all(feature="ownership", unix) => {
                lchown(path, uid, gid)?;
            },
            _ => ()
        };

        Ok(())
    }
}
