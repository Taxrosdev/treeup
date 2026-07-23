use async_trait::async_trait;
use std::{io, path::Path};
use stringlike::StringLike;
use tokio::fs;

#[cfg(all(any(feature = "mode", feature = "ownership"), unix))]
use crate::utils::permissions::Permissions;
use crate::{
    object::{Dependencies, Deployable, Object},
    repo::Repo,
};
mod file;
pub use file::File;
#[cfg(not(unix))]
mod symlink;
#[cfg(not(unix))]
pub use symlink::Symlink;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Tree {
    subtrees: Vec<SubtreeRef>,
    files: Vec<File>,
    #[cfg(not(unix))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    symlinks: Vec<Symlink>,

    #[cfg(all(feature = "mode", unix))]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    mode: Option<u32>,
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
impl Object for Tree {
    fn get_dependencies(&self) -> Dependencies<'_> {
        Dependencies {
            objects: self
                .subtrees
                .iter()
                .map(|tree| tree.hash.as_str())
                .collect(),
            blobs: self.files.iter().map(|file| &file.blob).collect(),
        }
    }
}

#[async_trait]
impl Deployable for Tree {
    async fn create(repo: &Repo, path: &Path) -> io::Result<Self> {
        #[cfg(all(any(feature = "mode", feature = "ownership"), unix))]
        let permissions = Permissions::get(path).await?;

        let mut subtrees = Vec::new();
        let mut files = Vec::new();
        #[cfg(not(unix))]
        let mut symlinks = Vec::new();

        let mut read_dir = fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let filetype = entry.file_type().await?;
            let filepath = entry.path();

            if filetype.is_dir() {
                let subtree = Tree::create(repo, &filepath).await?;
                let raw = serde_json::to_vec(&subtree)?;
                let hash = blake3::hash(&raw).to_string();

                subtrees.push(SubtreeRef {
                    hash,
                    name: filepath
                        .file_name()
                        .ok_or(io::ErrorKind::InvalidFilename)?
                        .to_os_string()
                        .into(),
                })
            } else if filetype.is_symlink() {
                #[cfg(not(unix))]
                let symlink = Symlink::create(repo, &filepath).await?;
                #[cfg(not(unix))]
                symlinks.push(symlink)
            } else if filetype.is_file() {
                let file = File::create(repo, &filepath).await?;
                files.push(file)
            }
        }

        let tree = Tree {
            subtrees,
            files,
            #[cfg(not(unix))]
            symlinks,
            #[cfg(all(feature = "ownership", unix))]
            uid: permissions.uid,
            #[cfg(all(feature = "ownership", unix))]
            gid: permissions.gid,
            #[cfg(all(feature = "mode", unix))]
            mode: permissions.mode,
        };

        let raw = serde_json::to_vec(&tree)?;
        let hash = blake3::hash(&raw).to_string();
        let object_path = Self::local_path(repo, &hash).await?;
        fs::write(object_path, raw).await?;

        Ok(tree)
    }

    async fn deploy(&self, repo: &Repo, deploy_path: &Path) -> io::Result<()> {
        fs::create_dir_all(deploy_path).await?;

        #[cfg(all(feature = "mode", unix))]
        Permissions::deploy_mode(deploy_path, self.mode).await?;
        #[cfg(all(feature = "ownership", unix))]
        Permissions::deploy_ownership(deploy_path, self.uid, self.gid).await?;

        // Subtrees
        for subtree in &self.subtrees {
            let tree = Tree::get(repo, &subtree.hash).await?;
            let path = deploy_path.join(subtree.name.to_path_buf());
            tree.deploy(repo, &path).await?;
        }

        // Files
        for file in &self.files {
            file.deploy(repo, deploy_path).await?;
        }

        // Symlinks
        #[cfg(not(unix))]
        for symlink in &self.symlinks {
            symlink.deploy(repo, deploy_path).await?;
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SubtreeRef {
    pub hash: String,
    pub name: StringLike,
}
