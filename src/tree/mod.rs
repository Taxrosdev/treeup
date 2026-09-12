use futures_util::{StreamExt, TryStreamExt, stream};
use std::{io, path::Path, path::PathBuf, sync::Arc};
use tokio::fs;
use treeup_core::object_cas::ObjectCAS;

use crate::object::Object;
use crate::utils::permissions::Permissions;
use crate::utils::stringlike::StringLike;
mod file;
pub use file::File;
mod symlink;
pub use symlink::Symlink;

/// HACK: This should be a configurable option.
const CREATE_FILES_CONCURRENCY: usize = 16;
/// HACK: This should be a configurable option.
const CREATE_SYMLINK_CONCURRENCY: usize = 4;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Tree {
    pub subtrees: Vec<SubtreeRef>,
    pub files: Vec<File>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub symlinks: Vec<Symlink>,

    #[serde(flatten)]
    #[serde(default)]
    permissions: Permissions,
}

impl Object for Tree {}

impl Tree {
    pub async fn create<C: ObjectCAS, P: AsRef<Path>>(
        cas: Arc<C>,
        blobs_path: &Path,
        path: P,
    ) -> io::Result<Self> {
        let permissions = Permissions::get(&path).await?;

        let mut subtrees = Vec::new();
        let mut files = Vec::new();
        let mut symlinks = Vec::new();

        let mut read_dir = fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let filetype = entry.file_type().await?;
            let filepath = entry.path();

            if filetype.is_dir() {
                let subtree = Box::pin(Tree::create(cas.clone(), blobs_path, &filepath)).await?;

                let raw = serde_json::to_string(&subtree)?;
                let hash = blake3::hash(raw.as_bytes()).to_string();

                subtrees.push(SubtreeRef {
                    hash,
                    name: filepath
                        .file_name()
                        .ok_or(io::ErrorKind::InvalidFilename)?
                        .to_os_string()
                        .into(),
                });
            } else if filetype.is_symlink() {
                symlinks.push(filepath);
            } else if filetype.is_file() {
                files.push(filepath);
            }
        }

        let files = stream::iter(files)
            .map(|path| File::create(blobs_path, path))
            .buffered(CREATE_FILES_CONCURRENCY)
            .try_collect::<Vec<_>>()
            .await?;
        let symlinks = stream::iter(symlinks)
            .map(Symlink::create)
            .buffered(CREATE_SYMLINK_CONCURRENCY)
            .try_collect::<Vec<_>>()
            .await?;

        let tree = Tree {
            subtrees,
            files,
            symlinks,
            permissions,
        };

        let raw = serde_json::to_string(&tree)?;
        let hash = blake3::hash(raw.as_bytes());
        cas.put(hash.as_slice(), &raw).await?;

        Ok(tree)
    }

    /// Will NOT deploy subdirectories. To get all subtrees, use `Tree::get_subtrees`
    ///
    /// A helper method `Tree::deploy_recursive` is available.
    pub async fn deploy(&self, blobs_path: &Path, deploy_path: &Path) -> io::Result<()> {
        fs::create_dir_all(deploy_path).await?;
        Permissions::deploy(
            deploy_path,
            self.permissions.mode,
            self.permissions.uid,
            self.permissions.gid,
        )
        .await?;

        // Files
        for file in &self.files {
            file.deploy(blobs_path, deploy_path).await?;
        }

        // Symlinks
        for symlink in &self.symlinks {
            symlink.deploy(deploy_path).await?;
        }

        Ok(())
    }
}

impl Tree {
    /// Will include self and (recursively) all decendants.
    /// It's guarrenteed that the parent will be ordered first before the children.
    pub async fn get_subtrees<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
    ) -> io::Result<Vec<(PathBuf, Tree)>> {
        let mut out = Vec::new();
        self.collect_subtrees(cas, PathBuf::from(""), &mut out)
            .await?;
        Ok(out)
    }

    async fn collect_subtrees<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        path: PathBuf,
        out: &mut Vec<(PathBuf, Tree)>,
    ) -> io::Result<()> {
        out.push((path.clone(), self.clone()));
        for subtree in &self.subtrees {
            let child_path = path.join(subtree.name.to_path_buf());
            let hash = hex::decode(&subtree.hash).map_err(io::Error::other)?;
            let tree = Tree::get(&*cas, &hash).await?;
            Box::pin(tree.collect_subtrees(cas.clone(), child_path, out)).await?;
        }
        Ok(())
    }

    pub async fn deploy_recursive<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blobs_path: &Path,
        deploy_path: &Path,
    ) -> io::Result<()> {
        for (sub_deploy_path, tree) in self.get_subtrees(cas.clone()).await? {
            let deploy_path = deploy_path.join(sub_deploy_path);
            tree.deploy(blobs_path, &deploy_path).await?;
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SubtreeRef {
    pub hash: String,
    pub name: StringLike,
}
