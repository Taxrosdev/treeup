use std::{path::PathBuf, sync::Arc};
use temp_dir::TempDir;
use tokio::fs;
use treeup::{
    Repo, Tree,
    object::{Deployable, Object, cas::BasicFS},
};

#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    let objects_path = tmp.path().join("objects");
    let blobs_path = tmp.path().join("blobs");
    let source_path = tmp.path().join("source");
    let deploy_path = tmp.path().join("deploy");

    // Create basic test data
    fs::create_dir_all(source_path.join("subdir")).await?;
    fs::write(source_path.join("text"), "hello world").await?;
    fs::write(source_path.join("data"), vec![0u8; 4096]).await?;
    fs::write(source_path.join("subdir").join("nested"), "nested content").await?;
    fs::symlink("text", source_path.join("link")).await?;

    let cas = Arc::new(BasicFS::create(objects_path.clone()).await?);
    let repo = Repo {
        objects_path: Arc::new(tmp.path().join("objects")),
        blobs_path: Arc::new(tmp.path().join("blobs")),
    };

    let tree = Tree::create(cas.clone(), &blobs_path, &source_path).await?;

    let hash = tree.hash()?;
    let hash_bytes = hex::decode(&hash)?;

    // Drop and Reopen
    drop(tree);
    drop(cas);
    drop(repo);

    let cas = Arc::new(BasicFS::create(objects_path).await?);
    let repo = Repo {
        objects_path: Arc::new(tmp.path().join("objects")),
        blobs_path: Arc::new(tmp.path().join("blobs")),
    };

    let tree = Tree::get(&*cas, &hash_bytes).await?;

    // Assert tree exists
    assert!(Tree::exists(&*cas, &hash_bytes).await?);

    // Assert tree has *at least* correct amount of subtrees/files/symlinks
    let retrieved = Tree::get(&*cas, &hash_bytes).await?;
    assert_eq!(retrieved.subtrees.len(), tree.subtrees.len());
    assert_eq!(retrieved.files.len(), tree.files.len());
    assert_eq!(retrieved.symlinks.len(), tree.symlinks.len());

    // Assert `Tree::get_dependencies` works
    let deps = tree.get_dependencies();
    assert_eq!(deps.objects.len(), 1);
    assert_eq!(deps.blobs.len(), 2);

    // Assert `BlobRef::exists` works
    for file in &tree.files {
        assert!(file.blob.exists(&repo).await?);
    }

    // Assert `Tree::get_subtrees` works
    let subtrees = tree.get_subtrees(cas.clone(), &blobs_path).await?;
    assert_eq!(subtrees.len(), 2);

    // Assert `Tree::deploy_recursive` works
    tree.deploy_recursive(cas.clone(), &blobs_path, &deploy_path)
        .await?;

    let content = fs::read(deploy_path.join("text")).await?;
    assert_eq!(content, b"hello world");

    let content = fs::read(deploy_path.join("data")).await?;
    assert_eq!(content, vec![0u8; 4096]);

    let content = fs::read(deploy_path.join("subdir").join("nested")).await?;
    assert_eq!(content, b"nested content");

    let target = fs::read_link(deploy_path.join("link")).await?;
    assert_eq!(target, PathBuf::from("text"));

    // Assert Cloning works
    let repo2 = Repo {
        objects_path: Arc::new(tmp.path().join("objects2")),
        blobs_path: Arc::new(tmp.path().join("blobs2")),
    };

    for file in &tree.files {
        let cloned = file.blob.try_clone(&repo, &repo2).await?;
        assert!(cloned);
        assert!(file.blob.exists(&repo2).await?);
    }

    let mut cas2 = BasicFS::create(tmp.path().join("objects2_cas")).await?;
    let cloned = Tree::try_clone(&*cas, &cas2, &hash_bytes).await?;
    assert!(cloned);
    assert!(Tree::exists(&cas2, &hash_bytes).await?);

    let cloned_tree = Tree::get(&cas2, &hash_bytes).await?;
    assert_eq!(cloned_tree.files.len(), tree.files.len());
    assert_eq!(cloned_tree.subtrees.len(), tree.subtrees.len());
    assert_eq!(cloned_tree.symlinks.len(), tree.symlinks.len());

    Ok(())
}
