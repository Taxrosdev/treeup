use async_trait::async_trait;
use std::{
    fs::Permissions,
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};
use stringlike::StringLike;
use tokio::fs;

use crate::{
    object::{Dependencies, Deployable, Object},
    repo::Repo,
};
mod file;
pub use file::File;
mod symlink;
pub use symlink::Symlink;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Tree {
    subtrees: Vec<SubtreeRef>,
    files: Vec<File>,
    symlinks: Vec<Symlink>,

    mode: u32,
    uid: u32,
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
        let metadata = fs::metadata(path).await?;

        // Permissions
        let uid = metadata.uid();
        let gid = if uid == metadata.gid() {
            None
        } else {
            Some(metadata.gid())
        };

        let mut subtrees = Vec::new();
        let mut files = Vec::new();
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
                    name: path
                        .file_name()
                        .ok_or(io::ErrorKind::InvalidFilename)?
                        .to_os_string()
                        .into(),
                })
            } else if filetype.is_symlink() {
                let symlink = Symlink::create(repo, &filepath).await?;
                symlinks.push(symlink)
            } else if filetype.is_file() {
                let file = File::create(repo, &filepath).await?;
                files.push(file)
            }
        }

        let tree = Tree {
            subtrees,
            files,
            symlinks,
            uid,
            gid,
            mode: metadata.mode(),
        };

        let raw = serde_json::to_vec(&tree)?;
        let hash = blake3::hash(&raw).to_string();
        let object_path = Self::local_path(repo, &hash).await?;
        fs::write(object_path, raw).await?;

        Ok(tree)
    }

    async fn deploy(&self, repo: &Repo, deploy_path: &Path) -> io::Result<()> {
        fs::create_dir_all(deploy_path).await?;

        // Permissions
        let gid = match self.gid {
            Some(gid) => gid,
            None => self.uid,
        };
        std::os::unix::fs::lchown(deploy_path, Some(self.uid), Some(gid))?;
        let permissions = Permissions::from_mode(self.mode);
        fs::set_permissions(deploy_path, permissions).await?;

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
