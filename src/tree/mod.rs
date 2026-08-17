use std::{io, path::Path, path::PathBuf, sync::Arc};
use tokio::fs;
use treeup_core::object_cas::ObjectCAS;

use crate::object::{Dependencies, Deployable, Object};
use crate::utils::permissions::Permissions;
use crate::utils::stringlike::StringLike;
mod file;
pub use file::File;
mod symlink;
pub use symlink::Symlink;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Tree {
    pub subtrees: Vec<SubtreeRef>,
    pub files: Vec<File>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub symlinks: Vec<Symlink>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    mode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    uid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    gid: Option<u32>,
}

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

impl Deployable for Tree {
    async fn create<C: ObjectCAS>(cas: Arc<C>, blobs_path: &Path, path: &Path) -> io::Result<Self> {
        let permissions = Permissions::get(path).await?;

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
                let symlink = Symlink::create(cas.clone(), blobs_path, &filepath).await?;
                symlinks.push(symlink);
            } else if filetype.is_file() {
                let file = File::create(cas.clone(), blobs_path, &filepath).await?;
                files.push(file);
            }
        }

        let tree = Tree {
            subtrees,
            files,
            symlinks,
            uid: permissions.uid,
            gid: permissions.gid,
            mode: permissions.mode,
        };

        let raw = serde_json::to_string(&tree)?;
        let hash = blake3::hash(raw.as_bytes());
        cas.put(hash.as_slice(), &raw).await?;

        Ok(tree)
    }

    /// Will NOT deploy subdirectories. To get all subtrees, use `Tree::get_subtrees`
    ///
    /// A helper method `Tree::deploy_recursive` is available.
    async fn deploy<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blobs_path: &Path,
        deploy_path: &Path,
    ) -> io::Result<()> {
        fs::create_dir_all(deploy_path).await?;
        Permissions::deploy(deploy_path.to_path_buf(), self.mode, self.uid, self.gid).await?;

        // Files
        for file in &self.files {
            let deploy_path = deploy_path.to_path_buf();
            file.deploy(cas.clone(), blobs_path, &deploy_path).await?;
        }

        // Symlinks
        for symlink in &self.symlinks {
            let symlink = symlink.clone();
            let deploy_path = deploy_path.to_path_buf();
            symlink
                .deploy(cas.clone(), blobs_path, &deploy_path)
                .await?;
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
        blobs_path: &Path,
    ) -> io::Result<Vec<(PathBuf, Tree)>> {
        let mut out = Vec::new();
        self.collect_subtrees(cas, blobs_path, PathBuf::from(""), &mut out)
            .await?;
        Ok(out)
    }

    async fn collect_subtrees<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blobs_path: &Path,
        path: PathBuf,
        out: &mut Vec<(PathBuf, Tree)>,
    ) -> io::Result<()> {
        out.push((path.clone(), self.clone()));
        for subtree in &self.subtrees {
            let child_path = path.join(subtree.name.to_path_buf());
            // TODO: Error handling
            let tree = Tree::get(&*cas.clone(), &hex::decode(&subtree.hash).unwrap()).await?;
            Box::pin(tree.collect_subtrees(cas.clone(), blobs_path, child_path, out)).await?;
        }
        Ok(())
    }

    pub async fn deploy_recursive<C: ObjectCAS>(
        &self,
        cas: Arc<C>,
        blobs_path: &Path,
        deploy_path: &Path,
    ) -> io::Result<()> {
        for (sub_deploy_path, tree) in self.get_subtrees(cas.clone(), blobs_path).await? {
            let deploy_path = deploy_path.join(sub_deploy_path);
            tree.deploy(cas.clone(), blobs_path, &deploy_path).await?;
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SubtreeRef {
    pub hash: String,
    pub name: StringLike,
}
